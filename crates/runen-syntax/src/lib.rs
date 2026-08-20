#![forbid(unsafe_code)]
//! Lossless syntax for the currently represented Runen concrete source subset.
//!
//! This crate implements accepted source text, token, and concrete grammar rules.
//! It intentionally performs no name resolution, source type checking, ownership
//! validation, HIR construction, Core MIR lowering, or runtime/backend work.

use std::fmt;

use rowan::{GreenNode, GreenNodeBuilder, Language, TextRange, TextSize};
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

    const fn is_type_start(self) -> bool {
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
    Identifier,
    Type,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Colon,
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
/// Invalid UTF-8 is rejected before lexing. For valid UTF-8, malformed concrete
/// syntax still produces a lossless tree plus diagnostics.
pub fn parse_source(bytes: &[u8]) -> Result<Parse, SourceInputError> {
    if bytes.len() > u32::MAX as usize {
        return Err(SourceInputError::TooLarge { len: bytes.len() });
    }

    let source = std::str::from_utf8(bytes).map_err(|error| SourceInputError::InvalidUtf8 {
        valid_up_to: error.valid_up_to(),
    })?;

    let (tokens, mut errors) = lex(source);
    let mut parser = Parser::new(source, tokens);
    parser.parse();
    errors.extend(parser.errors);
    Ok(Parse {
        green: parser.builder.finish(),
        errors,
    })
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

#[derive(Debug, Clone, Copy)]
struct LexToken {
    kind: SyntaxKind,
    start: usize,
    end: usize,
}

fn lex(source: &str) -> (Vec<LexToken>, Vec<SyntaxError>) {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();
    let mut offset = 0;

    if source.starts_with('\u{feff}') {
        let end = '\u{feff}'.len_utf8();
        tokens.push(LexToken {
            kind: SyntaxKind::Bom,
            start: 0,
            end,
        });
        offset = end;
    }

    while offset < source.len() {
        let rest = &source[offset..];

        if rest.starts_with("//") {
            let mut end = offset + 2;
            while end < source.len() {
                let character = source[end..].chars().next().expect("valid UTF-8");
                if is_line_boundary(character) {
                    break;
                }
                end += character.len_utf8();
            }
            tokens.push(LexToken {
                kind: SyntaxKind::LineComment,
                start: offset,
                end,
            });
            offset = end;
            continue;
        }

        if rest.starts_with("/*") {
            let mut end = offset + 2;
            let mut depth = 1_u32;
            while end < source.len() {
                let tail = &source[end..];
                if tail.starts_with("/*") {
                    depth += 1;
                    end += 2;
                } else if tail.starts_with("*/") {
                    depth -= 1;
                    end += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    end += tail.chars().next().expect("valid UTF-8").len_utf8();
                }
            }
            if depth != 0 {
                errors.push(SyntaxError {
                    kind: SyntaxErrorKind::UnterminatedBlockComment,
                    range: text_range(offset, source.len()),
                });
                end = source.len();
            }
            tokens.push(LexToken {
                kind: SyntaxKind::BlockComment,
                start: offset,
                end,
            });
            offset = end;
            continue;
        }

        let character = rest.chars().next().expect("non-empty source tail");

        if is_pattern_whitespace(character) {
            let mut end = offset + character.len_utf8();
            while end < source.len() {
                let next = source[end..].chars().next().expect("valid UTF-8");
                if !is_pattern_whitespace(next) {
                    break;
                }
                end += next.len_utf8();
            }
            tokens.push(LexToken {
                kind: SyntaxKind::Whitespace,
                start: offset,
                end,
            });
            offset = end;
            continue;
        }

        if character == '_' || unicode_ident::is_xid_start(character) {
            let mut end = offset + character.len_utf8();
            while end < source.len() {
                let next = source[end..].chars().next().expect("valid UTF-8");
                if next != '_' && !unicode_ident::is_xid_continue(next) {
                    break;
                }
                end += next.len_utf8();
            }
            let spelling = &source[offset..end];
            let key = spelling.nfc().collect::<String>();
            tokens.push(LexToken {
                kind: classify_identifier_key(&key),
                start: offset,
                end,
            });
            offset = end;
            continue;
        }

        let punctuation = if rest.starts_with("->") {
            Some((SyntaxKind::Arrow, 2))
        } else {
            match character {
                '(' => Some((SyntaxKind::LParen, 1)),
                ')' => Some((SyntaxKind::RParen, 1)),
                '{' => Some((SyntaxKind::LBrace, 1)),
                '}' => Some((SyntaxKind::RBrace, 1)),
                ':' => Some((SyntaxKind::Colon, 1)),
                ',' => Some((SyntaxKind::Comma, 1)),
                '=' => Some((SyntaxKind::Eq, 1)),
                ';' => Some((SyntaxKind::Semicolon, 1)),
                _ => None,
            }
        };

        if let Some((kind, width)) = punctuation {
            tokens.push(LexToken {
                kind,
                start: offset,
                end: offset + width,
            });
            offset += width;
            continue;
        }

        let end = offset + character.len_utf8();
        errors.push(SyntaxError {
            kind: SyntaxErrorKind::UnrecognizedToken,
            range: text_range(offset, end),
        });
        tokens.push(LexToken {
            kind: SyntaxKind::ErrorToken,
            start: offset,
            end,
        });
        offset = end;
    }

    (tokens, errors)
}

fn classify_identifier_key(key: &str) -> SyntaxKind {
    match key {
        "fn" => SyntaxKind::KwFn,
        "record" => SyntaxKind::KwRecord,
        "let" => SyntaxKind::KwLet,
        "return" => SyntaxKind::KwReturn,
        "Bool" => SyntaxKind::TyBool,
        "I8" => SyntaxKind::TyI8,
        "I16" => SyntaxKind::TyI16,
        "I32" => SyntaxKind::TyI32,
        "I64" => SyntaxKind::TyI64,
        "U8" => SyntaxKind::TyU8,
        "U16" => SyntaxKind::TyU16,
        "U32" => SyntaxKind::TyU32,
        "U64" => SyntaxKind::TyU64,
        "F16" => SyntaxKind::TyF16,
        "F32" => SyntaxKind::TyF32,
        "F64" => SyntaxKind::TyF64,
        _ => SyntaxKind::Ident,
    }
}

const fn is_pattern_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{0009}'
            | '\u{000a}'
            | '\u{000b}'
            | '\u{000c}'
            | '\u{000d}'
            | '\u{0020}'
            | '\u{0085}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

const fn is_line_boundary(character: char) -> bool {
    matches!(
        character,
        '\u{000a}'
            | '\u{000b}'
            | '\u{000c}'
            | '\u{000d}'
            | '\u{0085}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

fn text_range(start: usize, end: usize) -> TextRange {
    let start = u32::try_from(start).expect("source length prechecked");
    let end = u32::try_from(end).expect("source length prechecked");
    TextRange::new(TextSize::from(start), TextSize::from(end))
}

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<LexToken>,
    position: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<SyntaxError>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, tokens: Vec<LexToken>) -> Self {
        Self {
            source,
            tokens,
            position: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
        }
    }

    fn parse(&mut self) {
        self.builder.start_node(SyntaxKind::SourceUnit.into());
        self.bump_trivia();
        while self.position < self.tokens.len() {
            match self.current() {
                Some(SyntaxKind::KwRecord) => self.parse_record_definition(),
                Some(SyntaxKind::KwFn) => self.parse_function_definition(),
                Some(_) => {
                    self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Item));
                    self.recover_one();
                }
                None => break,
            }
            self.bump_trivia();
        }
        self.builder.finish_node();
    }

    fn parse_record_definition(&mut self) {
        self.builder
            .start_node(SyntaxKind::RecordDefinition.into());
        self.expect(SyntaxKind::KwRecord, ExpectedSyntax::Item);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);

        if !self.expect(SyntaxKind::LBrace, ExpectedSyntax::LeftBrace) {
            self.builder.finish_node();
            return;
        }

        self.bump_trivia();
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if self.at(SyntaxKind::Ident) {
                self.parse_record_field();
                if self.eat(SyntaxKind::Comma) {
                    self.bump_trivia();
                    continue;
                }
                if !self.at(SyntaxKind::RBrace) {
                    self.error_here(SyntaxErrorKind::Expected(
                        ExpectedSyntax::CommaOrRightBrace,
                    ));
                    self.recover_until(&[SyntaxKind::Comma, SyntaxKind::RBrace]);
                    self.eat(SyntaxKind::Comma);
                }
            } else {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Identifier));
                self.recover_one();
            }
            self.bump_trivia();
        }

        self.expect(SyntaxKind::RBrace, ExpectedSyntax::RightBrace);
        self.builder.finish_node();
    }

    fn parse_record_field(&mut self) {
        self.builder.start_node(SyntaxKind::RecordField.into());
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        self.expect(SyntaxKind::Colon, ExpectedSyntax::Colon);
        self.parse_type();
        self.builder.finish_node();
    }

    fn parse_function_definition(&mut self) {
        self.builder
            .start_node(SyntaxKind::FunctionDefinition.into());
        self.expect(SyntaxKind::KwFn, ExpectedSyntax::Item);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        self.parse_parameter_list();

        if self.at(SyntaxKind::Arrow) {
            self.builder.start_node(SyntaxKind::ResultClause.into());
            self.bump();
            self.parse_type();
            self.builder.finish_node();
        }

        self.parse_body();
        self.builder.finish_node();
    }

    fn parse_parameter_list(&mut self) {
        self.builder.start_node(SyntaxKind::ParameterList.into());
        if !self.expect(SyntaxKind::LParen, ExpectedSyntax::LeftParen) {
            self.builder.finish_node();
            return;
        }

        self.bump_trivia();
        while !self.at(SyntaxKind::RParen) && self.current().is_some() {
            if self.at(SyntaxKind::Ident) {
                self.builder.start_node(SyntaxKind::Parameter.into());
                self.bump();
                self.expect(SyntaxKind::Colon, ExpectedSyntax::Colon);
                self.parse_type();
                self.builder.finish_node();

                if self.eat(SyntaxKind::Comma) {
                    self.bump_trivia();
                    continue;
                }
                if !self.at(SyntaxKind::RParen) {
                    self.error_here(SyntaxErrorKind::Expected(
                        ExpectedSyntax::CommaOrRightParen,
                    ));
                    self.recover_until(&[SyntaxKind::Comma, SyntaxKind::RParen]);
                    self.eat(SyntaxKind::Comma);
                }
            } else {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Identifier));
                self.recover_one();
            }
            self.bump_trivia();
        }

        self.expect(SyntaxKind::RParen, ExpectedSyntax::RightParen);
        self.builder.finish_node();
    }

    fn parse_body(&mut self) {
        self.builder.start_node(SyntaxKind::Body.into());
        if !self.expect(SyntaxKind::LBrace, ExpectedSyntax::LeftBrace) {
            self.builder.finish_node();
            return;
        }

        self.bump_trivia();
        let mut returned = false;
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if returned {
                self.error_here(SyntaxErrorKind::UnexpectedAfterReturn);
                self.builder.start_node(SyntaxKind::ErrorNode.into());
                while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
                    self.bump();
                    self.bump_trivia();
                }
                self.builder.finish_node();
                break;
            }

            match self.current() {
                Some(SyntaxKind::KwLet) => self.parse_local_declaration(),
                Some(SyntaxKind::KwReturn) => {
                    self.parse_return_statement();
                    returned = true;
                }
                Some(SyntaxKind::Ident) => self.parse_call_statement(),
                Some(_) => {
                    self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Item));
                    self.recover_one();
                }
                None => break,
            }
            self.bump_trivia();
        }

        self.expect(SyntaxKind::RBrace, ExpectedSyntax::RightBrace);
        self.builder.finish_node();
    }

    fn parse_local_declaration(&mut self) {
        self.builder
            .start_node(SyntaxKind::LocalDeclaration.into());
        self.expect(SyntaxKind::KwLet, ExpectedSyntax::Item);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        self.expect(SyntaxKind::Colon, ExpectedSyntax::Colon);
        self.parse_type();
        self.expect(SyntaxKind::Eq, ExpectedSyntax::Equals);
        self.parse_value();
        self.expect(SyntaxKind::Semicolon, ExpectedSyntax::Semicolon);
        self.builder.finish_node();
    }

    fn parse_call_statement(&mut self) {
        self.builder.start_node(SyntaxKind::CallStatement.into());
        self.parse_direct_call();
        self.expect(SyntaxKind::Semicolon, ExpectedSyntax::Semicolon);
        self.builder.finish_node();
    }

    fn parse_return_statement(&mut self) {
        self.builder
            .start_node(SyntaxKind::ReturnStatement.into());
        self.expect(SyntaxKind::KwReturn, ExpectedSyntax::Item);
        if !self.at(SyntaxKind::Semicolon) {
            self.parse_value();
        }
        self.expect(SyntaxKind::Semicolon, ExpectedSyntax::Semicolon);
        self.builder.finish_node();
    }

    fn parse_type(&mut self) {
        self.builder.start_node(SyntaxKind::TypeRef.into());
        if self.current().is_some_and(SyntaxKind::is_type_start) {
            self.bump();
        } else {
            self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Type));
        }
        self.builder.finish_node();
    }

    fn parse_value(&mut self) {
        if !self.at(SyntaxKind::Ident) {
            self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Value));
            if self.current().is_some()
                && !self.at(SyntaxKind::Comma)
                && !self.at(SyntaxKind::RParen)
                && !self.at(SyntaxKind::Semicolon)
                && !self.at(SyntaxKind::RBrace)
            {
                self.recover_one();
            }
            return;
        }

        if self.peek_nontrivia(1) == Some(SyntaxKind::LParen) {
            self.parse_direct_call();
        } else {
            self.builder.start_node(SyntaxKind::IdentifierUse.into());
            self.bump();
            self.builder.finish_node();
        }
    }

    fn parse_direct_call(&mut self) {
        self.builder.start_node(SyntaxKind::DirectCall.into());
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);

        self.builder.start_node(SyntaxKind::ArgumentList.into());
        if !self.expect(SyntaxKind::LParen, ExpectedSyntax::LeftParen) {
            self.builder.finish_node();
            self.builder.finish_node();
            return;
        }

        self.bump_trivia();
        while !self.at(SyntaxKind::RParen) && self.current().is_some() {
            self.parse_value();

            if self.eat(SyntaxKind::Comma) {
                self.bump_trivia();
                continue;
            }
            if !self.at(SyntaxKind::RParen) {
                self.error_here(SyntaxErrorKind::Expected(
                    ExpectedSyntax::CommaOrRightParen,
                ));
                self.recover_until(&[SyntaxKind::Comma, SyntaxKind::RParen]);
                self.eat(SyntaxKind::Comma);
            }
            self.bump_trivia();
        }

        self.expect(SyntaxKind::RParen, ExpectedSyntax::RightParen);
        self.builder.finish_node();
        self.builder.finish_node();
    }

    fn current(&self) -> Option<SyntaxKind> {
        self.tokens[self.position..]
            .iter()
            .find(|token| !token.kind.is_trivia())
            .map(|token| token.kind)
    }

    fn peek_nontrivia(&self, index: usize) -> Option<SyntaxKind> {
        self.tokens[self.position..]
            .iter()
            .filter(|token| !token.kind.is_trivia())
            .nth(index)
            .map(|token| token.kind)
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == Some(kind)
    }

    fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: SyntaxKind, expected: ExpectedSyntax) -> bool {
        if self.eat(kind) {
            true
        } else {
            self.error_here(SyntaxErrorKind::Expected(expected));
            false
        }
    }

    fn bump(&mut self) {
        self.bump_trivia();
        if self.position < self.tokens.len() {
            self.bump_raw();
        }
    }

    fn bump_trivia(&mut self) {
        while self.position < self.tokens.len() && self.tokens[self.position].kind.is_trivia() {
            self.bump_raw();
        }
    }

    fn bump_raw(&mut self) {
        let token = self.tokens[self.position];
        let text = &self.source[token.start..token.end];
        self.builder.token(token.kind.into(), text);
        self.position += 1;
    }

    fn recover_one(&mut self) {
        self.builder.start_node(SyntaxKind::ErrorNode.into());
        self.bump();
        self.builder.finish_node();
    }

    fn recover_until(&mut self, stop: &[SyntaxKind]) {
        self.builder.start_node(SyntaxKind::ErrorNode.into());
        let start = self.position;
        while self.current().is_some_and(|kind| !stop.contains(&kind)) {
            self.bump();
        }
        if self.position == start && self.current().is_some() {
            self.bump();
        }
        self.builder.finish_node();
    }

    fn error_here(&mut self, kind: SyntaxErrorKind) {
        let range = self
            .tokens
            .get(self.position..)
            .and_then(|tokens| tokens.iter().find(|token| !token.kind.is_trivia()))
            .map_or_else(
                || text_range(self.source.len(), self.source.len()),
                |token| text_range(token.start, token.end),
            );
        self.errors.push(SyntaxError { kind, range });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Parse {
        parse_source(text.as_bytes()).expect("valid UTF-8 test source")
    }

    #[test]
    fn unicode_dependencies_are_pinned_to_17_0_0() {
        assert_eq!(unicode_ident::UNICODE_VERSION, (17, 0, 0));
        assert_eq!(unicode_normalization::UNICODE_VERSION, (17, 0, 0));
    }

    #[test]
    fn round_trips_empty_trivia_and_valid_source() {
        for source in [
            "",
            " \t// comment\r\n/* outer /* inner */ done */\n",
            "record Ticket { id: I64, }\nfn id(value: Ticket) -> Ticket { return value; }\n",
        ] {
            let parsed = parse(source);
            assert_eq!(parsed.text(), source);
        }
    }

    #[test]
    fn preserves_initial_bom_without_changing_keyword_tokenization() {
        let source = "\u{feff}fn id(value: I64) -> I64 { return value; }";
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        let kinds = parsed
            .syntax()
            .descendants_with_tokens()
            .filter_map(|element| element.into_token())
            .map(|token| token.kind())
            .collect::<Vec<_>>();
        assert_eq!(kinds.first(), Some(&SyntaxKind::Bom));
        assert!(kinds.contains(&SyntaxKind::KwFn));
    }

    #[test]
    fn rejects_invalid_utf8_before_parsing() {
        let error = parse_source(&[0xff]).expect_err("invalid UTF-8 must fail");
        assert_eq!(error, SourceInputError::InvalidUtf8 { valid_up_to: 0 });
    }

    #[test]
    fn maximal_identifier_extent_precedes_reserved_key_classification() {
        let parsed = parse("fnx(value: I64) {}");
        let tokens = parsed
            .syntax()
            .descendants_with_tokens()
            .filter_map(|element| element.into_token())
            .collect::<Vec<_>>();
        assert_eq!(tokens[0].kind(), SyntaxKind::Ident);
        assert_eq!(tokens[0].text(), "fnx");
    }

    #[test]
    fn derives_nfc_equivalent_identifier_keys() {
        let decomposed = "e\u{301}";
        let composed = "é";
        assert_eq!(identifier_key(decomposed), identifier_key(composed));
        assert_eq!(identifier_key(composed).as_deref(), Some(composed));
        assert_eq!(identifier_key("fn"), Some("fn".to_owned()));
        assert_eq!(identifier_key("1bad"), None);
    }

    #[test]
    fn handles_pattern_whitespace_and_logical_line_comments() {
        let source = "fn\u{200e}a() {//x\r\n//y\u{2028}return;\n}";
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed
            .errors()
            .iter()
            .any(|error| error.kind() == SyntaxErrorKind::UnrecognizedToken));
    }

    #[test]
    fn handles_nested_and_unterminated_block_comments() {
        let nested = parse("/* outer /* inner */ done */ fn a() {}");
        assert_eq!(nested.text(), "/* outer /* inner */ done */ fn a() {}");
        assert!(!nested
            .errors()
            .iter()
            .any(|error| error.kind() == SyntaxErrorKind::UnterminatedBlockComment));

        let source = "fn a() { /* never closes";
        let unterminated = parse(source);
        assert_eq!(unterminated.text(), source);
        assert!(unterminated
            .errors()
            .iter()
            .any(|error| error.kind() == SyntaxErrorKind::UnterminatedBlockComment));
    }

    #[test]
    fn unsupported_concrete_text_is_retained_as_error_tokens() {
        let source = "fn a() { let x: I64 = 42; + . }";
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(parsed
            .errors()
            .iter()
            .any(|error| error.kind() == SyntaxErrorKind::UnrecognizedToken));
    }

    #[test]
    fn parses_representative_records_functions_locals_calls_and_returns() {
        let source = r#"
record Ticket { id: I64, }
fn identity(value: Ticket) -> Ticket {
    return value;
}
fn forward(value: Ticket) -> Ticket {
    let moved: Ticket = identity(value,);
    return identity(moved);
}
fn sink(value: Ticket) {
    consume(value);
}
fn consume(value: Ticket,) {}
"#;
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

        let root = parsed.syntax();
        assert_eq!(
            root.children()
                .filter(|node| node.kind() == SyntaxKind::FunctionDefinition)
                .count(),
            4
        );
    }

    #[test]
    fn malformed_input_recovers_without_losing_text() {
        let source = "record A { x I64, y: I64 } fn broken( { let x: I64 = ; return; trailing } fn ok() {}";
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(parsed.errors().len() >= 3);
        assert!(parsed
            .syntax()
            .descendants()
            .any(|node| node.kind() == SyntaxKind::FunctionDefinition));
    }
}
