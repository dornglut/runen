use crate::{
    SyntaxError, SyntaxErrorKind, SyntaxKind, identifier_key, reserved_identifier_kind, text_range,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct LexToken {
    pub(crate) kind: SyntaxKind,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) fn lex(source: &str) -> (Vec<LexToken>, Vec<SyntaxError>) {
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
                errors.push(SyntaxError::new(
                    SyntaxErrorKind::UnterminatedBlockComment,
                    text_range(offset, source.len()),
                ));
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
            let key = identifier_key(spelling).expect("lexer established identifier form");
            tokens.push(LexToken {
                kind: reserved_identifier_kind(&key).unwrap_or(SyntaxKind::Ident),
                start: offset,
                end,
            });
            offset = end;
            continue;
        }

        if character.is_ascii_digit() {
            let mut end = offset + 1;
            while end < source.len() && source.as_bytes()[end].is_ascii_digit() {
                end += 1;
            }
            tokens.push(LexToken {
                kind: SyntaxKind::DecimalMagnitude,
                start: offset,
                end,
            });
            offset = end;
            continue;
        }

        let punctuation = if rest.starts_with("->") {
            Some((SyntaxKind::Arrow, 2))
        } else if rest.starts_with("::") {
            Some((SyntaxKind::ColonColon, 2))
        } else {
            match character {
                '(' => Some((SyntaxKind::LParen, 1)),
                ')' => Some((SyntaxKind::RParen, 1)),
                '{' => Some((SyntaxKind::LBrace, 1)),
                '}' => Some((SyntaxKind::RBrace, 1)),
                ':' => Some((SyntaxKind::Colon, 1)),
                ',' => Some((SyntaxKind::Comma, 1)),
                '-' => Some((SyntaxKind::Minus, 1)),
                '=' => Some((SyntaxKind::Eq, 1)),
                ';' => Some((SyntaxKind::Semicolon, 1)),
                '.' => Some((SyntaxKind::Dot, 1)),
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
        errors.push(SyntaxError::new(
            SyntaxErrorKind::UnrecognizedToken,
            text_range(offset, end),
        ));
        tokens.push(LexToken {
            kind: SyntaxKind::ErrorToken,
            start: offset,
            end,
        });
        offset = end;
    }

    (tokens, errors)
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
        '\u{000a}' | '\u{000b}' | '\u{000c}' | '\u{000d}' | '\u{0085}' | '\u{2028}' | '\u{2029}'
    )
}
