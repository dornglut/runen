use rowan::{GreenNode, GreenNodeBuilder};

use crate::{
    ExpectedSyntax, SyntaxError, SyntaxErrorKind, SyntaxKind, lexer::LexToken, text_range,
};

const TOP_LEVEL_STARTERS: &[SyntaxKind] = &[
    SyntaxKind::KwImport,
    SyntaxKind::KwExport,
    SyntaxKind::KwFn,
    SyntaxKind::KwRecord,
];

pub(crate) fn parse(source: &str, tokens: Vec<LexToken>) -> (GreenNode, Vec<SyntaxError>) {
    let mut parser = Parser {
        source,
        tokens,
        position: 0,
        builder: GreenNodeBuilder::new(),
        errors: Vec::new(),
    };
    parser.parse_root();
    (parser.builder.finish(), parser.errors)
}

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<LexToken>,
    position: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<SyntaxError>,
}

impl Parser<'_> {
    fn parse_root(&mut self) {
        self.builder.start_node(SyntaxKind::SourceUnit.into());
        self.bump_trivia();
        while self.position < self.tokens.len() {
            match self.current() {
                Some(SyntaxKind::KwImport) => self.parse_import_declaration(),
                Some(SyntaxKind::KwExport) => match self.peek_nontrivia(1) {
                    Some(SyntaxKind::KwRecord) => self.parse_record_definition(true),
                    Some(SyntaxKind::KwFn) => self.parse_function_definition(true),
                    _ => {
                        self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Item));
                        self.recover_one();
                    }
                },
                Some(SyntaxKind::KwRecord) => self.parse_record_definition(false),
                Some(SyntaxKind::KwFn) => self.parse_function_definition(false),
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

    fn parse_import_declaration(&mut self) {
        self.builder
            .start_node(SyntaxKind::ImportDeclaration.into());
        self.expect(SyntaxKind::KwImport, ExpectedSyntax::Item);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        self.expect(SyntaxKind::Semicolon, ExpectedSyntax::Semicolon);
        self.builder.finish_node();
    }

    fn parse_record_definition(&mut self, exported: bool) {
        self.builder.start_node(SyntaxKind::RecordDefinition.into());
        if exported {
            self.expect(SyntaxKind::KwExport, ExpectedSyntax::Item);
        }
        self.expect(SyntaxKind::KwRecord, ExpectedSyntax::Item);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);

        if !self.expect(SyntaxKind::LBrace, ExpectedSyntax::LeftBrace) {
            self.builder.finish_node();
            return;
        }

        self.bump_trivia();
        let mut missing_close = false;
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if self.at_any(TOP_LEVEL_STARTERS) {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::RightBrace));
                missing_close = true;
                break;
            }

            if self.at(SyntaxKind::Ident) {
                self.parse_record_field();
                if self.eat(SyntaxKind::Comma) {
                    self.bump_trivia();
                    continue;
                }
                if !self.at(SyntaxKind::RBrace) {
                    self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::CommaOrRightBrace));
                    self.recover_until(&[
                        SyntaxKind::Comma,
                        SyntaxKind::RBrace,
                        SyntaxKind::KwImport,
                        SyntaxKind::KwExport,
                        SyntaxKind::KwFn,
                        SyntaxKind::KwRecord,
                    ]);
                    self.eat(SyntaxKind::Comma);
                }
            } else {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Identifier));
                self.recover_one();
            }
            self.bump_trivia();
        }

        if !missing_close {
            self.expect(SyntaxKind::RBrace, ExpectedSyntax::RightBrace);
        }
        self.builder.finish_node();
    }

    fn parse_record_field(&mut self) {
        self.builder.start_node(SyntaxKind::RecordField.into());
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        self.expect(SyntaxKind::Colon, ExpectedSyntax::Colon);
        self.parse_type();
        self.builder.finish_node();
    }

    fn parse_function_definition(&mut self, exported: bool) {
        self.builder
            .start_node(SyntaxKind::FunctionDefinition.into());
        if exported {
            self.expect(SyntaxKind::KwExport, ExpectedSyntax::Item);
        }
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
        let mut missing_close = false;
        while !self.at(SyntaxKind::RParen) && self.current().is_some() {
            if self.at(SyntaxKind::Arrow)
                || self.at(SyntaxKind::LBrace)
                || self.at_any(TOP_LEVEL_STARTERS)
            {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::RightParen));
                missing_close = true;
                break;
            }

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
                    self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::CommaOrRightParen));
                    self.recover_until(&[
                        SyntaxKind::Comma,
                        SyntaxKind::RParen,
                        SyntaxKind::Arrow,
                        SyntaxKind::LBrace,
                        SyntaxKind::KwImport,
                        SyntaxKind::KwExport,
                        SyntaxKind::KwFn,
                        SyntaxKind::KwRecord,
                    ]);
                    self.eat(SyntaxKind::Comma);
                }
            } else {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Identifier));
                self.recover_one();
            }
            self.bump_trivia();
        }

        if !missing_close {
            self.expect(SyntaxKind::RParen, ExpectedSyntax::RightParen);
        }
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
        let mut missing_close = false;
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if self.at_any(TOP_LEVEL_STARTERS) {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::RightBrace));
                missing_close = true;
                break;
            }

            if returned {
                self.error_here(SyntaxErrorKind::UnexpectedAfterReturn);
                self.builder.start_node(SyntaxKind::ErrorNode.into());
                while !self.at(SyntaxKind::RBrace)
                    && !self.at_any(TOP_LEVEL_STARTERS)
                    && self.current().is_some()
                {
                    self.bump();
                    self.bump_trivia();
                }
                self.builder.finish_node();
                if self.at_any(TOP_LEVEL_STARTERS) {
                    self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::RightBrace));
                    missing_close = true;
                }
                break;
            }

            match self.current() {
                Some(SyntaxKind::KwLet) => self.parse_local_declaration(),
                Some(SyntaxKind::KwReturn) => {
                    self.parse_return_statement();
                    returned = true;
                }
                Some(SyntaxKind::Ident) => self.parse_identifier_statement(),
                Some(_) => {
                    self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Statement));
                    self.recover_one();
                }
                None => break,
            }
            self.bump_trivia();
        }

        if !missing_close {
            self.expect(SyntaxKind::RBrace, ExpectedSyntax::RightBrace);
        }
        self.builder.finish_node();
    }

    fn parse_local_declaration(&mut self) {
        self.builder.start_node(SyntaxKind::LocalDeclaration.into());
        self.expect(SyntaxKind::KwLet, ExpectedSyntax::Item);
        self.eat(SyntaxKind::KwMut);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        self.expect(SyntaxKind::Colon, ExpectedSyntax::Colon);
        self.parse_type();
        self.expect(SyntaxKind::Eq, ExpectedSyntax::Equals);
        self.parse_value();
        self.expect(SyntaxKind::Semicolon, ExpectedSyntax::Semicolon);
        self.builder.finish_node();
    }

    fn parse_identifier_statement(&mut self) {
        match self.peek_nontrivia(1) {
            Some(SyntaxKind::Eq) => self.parse_assignment_statement(),
            Some(SyntaxKind::LParen | SyntaxKind::ColonColon) => self.parse_call_statement(),
            _ => {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Statement));
                self.recover_until(&[
                    SyntaxKind::Semicolon,
                    SyntaxKind::RBrace,
                    SyntaxKind::KwLet,
                    SyntaxKind::KwReturn,
                    SyntaxKind::KwImport,
                    SyntaxKind::KwExport,
                    SyntaxKind::KwFn,
                    SyntaxKind::KwRecord,
                ]);
                self.eat(SyntaxKind::Semicolon);
            }
        }
    }

    fn parse_assignment_statement(&mut self) {
        self.builder
            .start_node(SyntaxKind::AssignmentStatement.into());
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
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
        self.builder.start_node(SyntaxKind::ReturnStatement.into());
        self.expect(SyntaxKind::KwReturn, ExpectedSyntax::Item);
        if self.at(SyntaxKind::Ident) {
            self.parse_value();
        }
        self.expect(SyntaxKind::Semicolon, ExpectedSyntax::Semicolon);
        self.builder.finish_node();
    }

    fn parse_type(&mut self) {
        self.builder.start_node(SyntaxKind::TypeRef.into());
        if self.at(SyntaxKind::Ident) && self.peek_nontrivia(1) == Some(SyntaxKind::ColonColon) {
            self.parse_qualified_module_member();
        } else if self.current().is_some_and(SyntaxKind::is_type_start) {
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
                && !self.at_any(&[
                    SyntaxKind::Comma,
                    SyntaxKind::RParen,
                    SyntaxKind::Semicolon,
                    SyntaxKind::RBrace,
                    SyntaxKind::KwLet,
                    SyntaxKind::KwReturn,
                    SyntaxKind::KwImport,
                    SyntaxKind::KwExport,
                    SyntaxKind::KwFn,
                    SyntaxKind::KwRecord,
                ])
            {
                self.recover_one();
            }
            return;
        }

        if matches!(
            self.peek_nontrivia(1),
            Some(SyntaxKind::LParen | SyntaxKind::ColonColon)
        ) {
            self.parse_direct_call();
        } else {
            self.builder.start_node(SyntaxKind::IdentifierUse.into());
            self.bump();
            self.builder.finish_node();
        }
    }

    fn parse_direct_call(&mut self) {
        self.builder.start_node(SyntaxKind::DirectCall.into());
        if self.at(SyntaxKind::Ident) && self.peek_nontrivia(1) == Some(SyntaxKind::ColonColon) {
            self.parse_qualified_module_member();
        } else {
            self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        }

        self.builder.start_node(SyntaxKind::ArgumentList.into());
        if !self.expect(SyntaxKind::LParen, ExpectedSyntax::LeftParen) {
            self.builder.finish_node();
            self.builder.finish_node();
            return;
        }

        self.bump_trivia();
        let mut missing_close = false;
        while !self.at(SyntaxKind::RParen) && self.current().is_some() {
            if self.at(SyntaxKind::Semicolon)
                || self.at(SyntaxKind::RBrace)
                || self.at(SyntaxKind::KwLet)
                || self.at(SyntaxKind::KwReturn)
                || self.at_any(TOP_LEVEL_STARTERS)
            {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::RightParen));
                missing_close = true;
                break;
            }

            self.parse_value();

            if self.eat(SyntaxKind::Comma) {
                self.bump_trivia();
                continue;
            }
            if !self.at(SyntaxKind::RParen) {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::CommaOrRightParen));
                self.recover_until(&[
                    SyntaxKind::Comma,
                    SyntaxKind::RParen,
                    SyntaxKind::Semicolon,
                    SyntaxKind::RBrace,
                    SyntaxKind::KwLet,
                    SyntaxKind::KwReturn,
                    SyntaxKind::KwImport,
                    SyntaxKind::KwExport,
                    SyntaxKind::KwFn,
                    SyntaxKind::KwRecord,
                ]);
                self.eat(SyntaxKind::Comma);
            }
            self.bump_trivia();
        }

        if !missing_close {
            self.expect(SyntaxKind::RParen, ExpectedSyntax::RightParen);
        }
        self.builder.finish_node();
        self.builder.finish_node();
    }

    fn parse_qualified_module_member(&mut self) {
        self.builder
            .start_node(SyntaxKind::QualifiedModuleMember.into());
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        self.expect(SyntaxKind::ColonColon, ExpectedSyntax::DoubleColon);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
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

    fn at_any(&self, kinds: &[SyntaxKind]) -> bool {
        self.current().is_some_and(|kind| kinds.contains(&kind))
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
        if self.current().is_none_or(|kind| stop.contains(&kind)) {
            return;
        }

        self.builder.start_node(SyntaxKind::ErrorNode.into());
        while self.current().is_some_and(|kind| !stop.contains(&kind)) {
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
        self.errors.push(SyntaxError::new(kind, range));
    }
}
