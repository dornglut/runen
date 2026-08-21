#![forbid(unsafe_code)]
//! Lossless syntax for the currently represented Runen concrete source subset.
//!
//! This crate implements accepted source text, token, and concrete grammar rules.
//! It intentionally performs no name resolution, source type checking, ownership
//! validation, HIR construction, Core MIR lowering, or runtime/backend work.

mod lexer;
mod parser;

use std::fmt;

use rowan::{GreenNode, Language};
pub use rowan::{TextRange, TextSize};
use unicode_normalization::UnicodeNormalization;

/// Rowan language marker for Runen concrete syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RunenLanguage {}

/// Token and node kinds represented by the first concrete syntax layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    Bom = 0,
    Whitespace,
    LineComment,
    BlockComment,
    Ident,
    KwFn,
    KwRecord,
    KwLet,
    KwReturn,
    TyBool,
    TyI8,
    TyI16,
    TyI32,
    TyI64,
    TyU8,
    TyU16,
    TyU32,
    TyU64,
    TyF16,
    TyF32,
    TyF64,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Colon,
    Comma,
    Arrow,
    Eq,
    Semicolon,
    ErrorToken,
    SourceUnit,
    RecordDefinition,
    RecordField,
    TypeRef,
    FunctionDefinition,
    ParameterList,
    Parameter,
    ResultClause,
    Body,
    LocalDeclaration,
    DirectCall,
    ArgumentList,
    CallStatement,
    IdentifierUse,
    ReturnStatement,
    ErrorNode,
    KwImport,
    KwExport,
    ColonColon,
    ImportDeclaration,
    QualifiedModuleMember,
}

impl SyntaxKind {
    /// Whether this kind is semantically inert source trivia.
    #[must_use]
    pub const fn is_trivia(self) -> bool {
        matches!(
            self,
            Self::Bom | Self::Whitespace | Self::LineComment | Self::BlockComment
        )
    }

    pub(crate) const fn is_type_start(self) -> bool {
        matches!(
            self,
            Self::Ident
                | Self::TyBool
                | Self::TyI8
                | Self::TyI16
                | Self::TyI32
                | Self::TyI64
                | Self::TyU8
                | Self::TyU16
                | Self::TyU32
                | Self::TyU64
                | Self::TyF16
                | Self::TyF32
                | Self::TyF64
        )
    }
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

impl Language for RunenLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        match raw.0 {
            0 => SyntaxKind::Bom,
            1 => SyntaxKind::Whitespace,
            2 => SyntaxKind::LineComment,
            3 => SyntaxKind::BlockComment,
            4 => SyntaxKind::Ident,
            5 => SyntaxKind::KwFn,
            6 => SyntaxKind::KwRecord,
            7 => SyntaxKind::KwLet,
            8 => SyntaxKind::KwReturn,
            9 => SyntaxKind::TyBool,
            10 => SyntaxKind::TyI8,
            11 => SyntaxKind::TyI16,
            12 => SyntaxKind::TyI32,
            13 => SyntaxKind::TyI64,
            14 => SyntaxKind::TyU8,
            15 => SyntaxKind::TyU16,
            16 => SyntaxKind::TyU32,
            17 => SyntaxKind::TyU64,
            18 => SyntaxKind::TyF16,
            19 => SyntaxKind::TyF32,
            20 => SyntaxKind::TyF64,
            21 => SyntaxKind::LParen,
            22 => SyntaxKind::RParen,
            23 => SyntaxKind::LBrace,
            24 => SyntaxKind::RBrace,
            25 => SyntaxKind::Colon,
            26 => SyntaxKind::Comma,
            27 => SyntaxKind::Arrow,
            28 => SyntaxKind::Eq,
            29 => SyntaxKind::Semicolon,
            30 => SyntaxKind::ErrorToken,
            31 => SyntaxKind::SourceUnit,
            32 => SyntaxKind::RecordDefinition,
            33 => SyntaxKind::RecordField,
            34 => SyntaxKind::TypeRef,
            35 => SyntaxKind::FunctionDefinition,
            36 => SyntaxKind::ParameterList,
            37 => SyntaxKind::Parameter,
            38 => SyntaxKind::ResultClause,
            39 => SyntaxKind::Body,
            40 => SyntaxKind::LocalDeclaration,
            41 => SyntaxKind::DirectCall,
            42 => SyntaxKind::ArgumentList,
            43 => SyntaxKind::CallStatement,
            44 => SyntaxKind::IdentifierUse,
            45 => SyntaxKind::ReturnStatement,
            46 => SyntaxKind::ErrorNode,
            47 => SyntaxKind::KwImport,
            48 => SyntaxKind::KwExport,
            49 => SyntaxKind::ColonColon,
            50 => SyntaxKind::ImportDeclaration,
            51 => SyntaxKind::QualifiedModuleMember,
            other => panic!("unknown Runen syntax kind {other}"),
        }
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

/// A typed Runen syntax node.
pub type SyntaxNode = rowan::SyntaxNode<RunenLanguage>;
/// A typed Runen syntax token.
pub type SyntaxToken = rowan::SyntaxToken<RunenLanguage>;
/// A typed Runen syntax node-or-token.
pub type SyntaxElement = rowan::NodeOrToken<SyntaxNode, SyntaxToken>;

/// Failure before a lossless syntax tree can be constructed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceInputError {
    /// The source byte sequence is not valid UTF-8.
    InvalidUtf8 { valid_up_to: usize },
    /// The source is valid Runen text but exceeds this implementation's tree range capacity.
    TooLarge { len: usize },
}

impl fmt::Display for SourceInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUtf8 { valid_up_to } => {
                write!(formatter, "invalid UTF-8 at byte offset {valid_up_to}")
            }
            Self::TooLarge { len } => write!(
                formatter,
                "source length {len} exceeds the syntax implementation capacity"
            ),
        }
    }
}

impl std::error::Error for SourceInputError {}

/// Structured syntax diagnostic category.
///
/// Display wording is intentionally not a stable compatibility contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxErrorKind {
    UnrecognizedToken,
    UnterminatedBlockComment,
    Expected(ExpectedSyntax),
    UnexpectedAfterReturn,
}

/// Coarse expected-syntax category used by parser diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedSyntax {
    Item,
    Statement,
    Identifier,
    Type,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Colon,
    DoubleColon,
    CommaOrRightParen,
    CommaOrRightBrace,
    Equals,
    Semicolon,
    Value,
}

/// One syntax diagnostic with a source byte range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxError {
    kind: SyntaxErrorKind,
    range: TextRange,
}

impl SyntaxError {
    pub(crate) const fn new(kind: SyntaxErrorKind, range: TextRange) -> Self {
        Self { kind, range }
    }

    #[must_use]
    pub const fn kind(&self) -> SyntaxErrorKind {
        self.kind
    }

    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }
}

/// Lossless parse result for valid UTF-8 source input.
#[derive(Debug, Clone)]
pub struct Parse {
    green: GreenNode,
    errors: Vec<SyntaxError>,
}

impl Parse {
    /// Root syntax node.
    #[must_use]
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }

    /// Structured lexer/parser diagnostics.
    #[must_use]
    pub fn errors(&self) -> &[SyntaxError] {
        &self.errors
    }

    /// Exact source text reconstructed from all tree tokens.
    #[must_use]
    pub fn text(&self) -> String {
        self.syntax().to_string()
    }
}

/// Parse one raw Runen source-unit byte sequence.
///
/// Invalid UTF-8 is rejected before capacity checking or lexing. For valid UTF-8,
/// malformed concrete syntax still produces a lossless tree plus diagnostics.
pub fn parse_source(bytes: &[u8]) -> Result<Parse, SourceInputError> {
    let source = std::str::from_utf8(bytes).map_err(|error| SourceInputError::InvalidUtf8 {
        valid_up_to: error.valid_up_to(),
    })?;

    if bytes.len() > u32::MAX as usize {
        return Err(SourceInputError::TooLarge { len: bytes.len() });
    }

    let (tokens, mut errors) = lexer::lex(source);
    let (green, parser_errors) = parser::parse(source, tokens);
    errors.extend(parser_errors);
    Ok(Parse { green, errors })
}

/// Derive the accepted NFC lexical identifier key for a complete identifier-form spelling.
///
/// Returns `None` when the whole string is not one accepted identifier-form token.
#[must_use]
pub fn identifier_key(text: &str) -> Option<String> {
    let mut chars = text.chars();
    let first = chars.next()?;
    if first != '_' && !unicode_ident::is_xid_start(first) {
        return None;
    }
    if chars.any(|character| character != '_' && !unicode_ident::is_xid_continue(character)) {
        return None;
    }
    Some(text.nfc().collect())
}

/// Derive the lexical key for one concrete `UserIdentifier` spelling.
///
/// Reserved concrete keys are rejected after the complete identifier key has
/// been formed, matching lexer classification.
#[must_use]
pub fn user_identifier_key(text: &str) -> Option<String> {
    let key = identifier_key(text)?;
    if reserved_identifier_kind(&key).is_some() {
        None
    } else {
        Some(key)
    }
}

pub(crate) fn reserved_identifier_kind(key: &str) -> Option<SyntaxKind> {
    match key {
        "fn" => Some(SyntaxKind::KwFn),
        "record" => Some(SyntaxKind::KwRecord),
        "let" => Some(SyntaxKind::KwLet),
        "return" => Some(SyntaxKind::KwReturn),
        "import" => Some(SyntaxKind::KwImport),
        "export" => Some(SyntaxKind::KwExport),
        "Bool" => Some(SyntaxKind::TyBool),
        "I8" => Some(SyntaxKind::TyI8),
        "I16" => Some(SyntaxKind::TyI16),
        "I32" => Some(SyntaxKind::TyI32),
        "I64" => Some(SyntaxKind::TyI64),
        "U8" => Some(SyntaxKind::TyU8),
        "U16" => Some(SyntaxKind::TyU16),
        "U32" => Some(SyntaxKind::TyU32),
        "U64" => Some(SyntaxKind::TyU64),
        "F16" => Some(SyntaxKind::TyF16),
        "F32" => Some(SyntaxKind::TyF32),
        "F64" => Some(SyntaxKind::TyF64),
        _ => None,
    }
}

pub(crate) fn text_range(start: usize, end: usize) -> TextRange {
    let start = u32::try_from(start).expect("source length prechecked");
    let end = u32::try_from(end).expect("source length prechecked");
    TextRange::new(TextSize::from(start), TextSize::from(end))
}
