use rowan::{GreenNode, GreenNodeBuilder};

use crate::{
    ExpectedSyntax, SyntaxError, SyntaxErrorKind, SyntaxKind, identifier_key, lexer::LexToken,
    text_range,
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

#[derive(Clone, Copy)]
enum ValueContext {
    Ordinary,
    Conditional,
}

#[derive(Clone, Copy)]
enum RecordPatternContext {
    Irrefutable,
    Refutable,
}

impl Parser<'_> {
    fn parse_root(&mut self) {
        self.builder.start_node(SyntaxKind::SourceUnit.into());
        self.bump_trivia();
        while self.position < self.tokens.len() {
            match self.current() {
                Some(SyntaxKind::KwImport) => self.parse_import_declaration(),
                Some(SyntaxKind::KwExport) if self.at_contextual_ident(1, "trait") => {
                    self.parse_trait_declaration(true);
                }
                Some(SyntaxKind::KwExport) if self.at_contextual_ident(1, "const") => {
                    self.parse_const_declaration(true);
                }
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
                Some(SyntaxKind::Ident) if self.at_contextual_ident(0, "trait") => {
                    self.parse_trait_declaration(false);
                }
                Some(SyntaxKind::Ident) if self.at_contextual_ident(0, "impl") => {
                    self.parse_trait_implementation();
                }
                Some(SyntaxKind::Ident) if self.at_contextual_ident(0, "const") => {
                    self.parse_const_declaration(false);
                }
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

    fn parse_const_declaration(&mut self, exported: bool) {
        debug_assert!(self.at_contextual_ident(usize::from(exported), "const"));
        self.builder.start_node(SyntaxKind::ConstDeclaration.into());
        if exported {
            self.expect(SyntaxKind::KwExport, ExpectedSyntax::Item);
        }
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Item);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        self.expect(SyntaxKind::Colon, ExpectedSyntax::Colon);
        self.parse_const_type();
        self.expect(SyntaxKind::Eq, ExpectedSyntax::Equals);
        if !self.parse_const_literal() {
            self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Value));
            if self.current().is_some() && !self.at(SyntaxKind::Semicolon) {
                self.recover_one();
            }
        }
        self.expect(SyntaxKind::Semicolon, ExpectedSyntax::Semicolon);
        self.builder.finish_node();
    }

    fn parse_const_type(&mut self) {
        self.builder.start_node(SyntaxKind::TypeRef.into());
        if self.current().is_some_and(is_intrinsic_type_start) {
            self.bump();
        } else {
            self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Type));
            if self.current().is_some() && !self.at(SyntaxKind::Eq) {
                self.recover_one();
            }
        }
        self.builder.finish_node();
    }

    fn parse_const_literal(&mut self) -> bool {
        match self.current() {
            Some(SyntaxKind::KwTrue | SyntaxKind::KwFalse) => {
                self.builder.start_node(SyntaxKind::BooleanLiteral.into());
                self.bump();
                self.builder.finish_node();
                true
            }
            Some(SyntaxKind::DecimalMagnitude) => {
                self.builder
                    .start_node(SyntaxKind::DecimalIntegerLiteral.into());
                self.bump();
                self.builder.finish_node();
                true
            }
            Some(SyntaxKind::DecimalFloatingMagnitude) => {
                self.builder
                    .start_node(SyntaxKind::DecimalFloatingLiteral.into());
                self.bump();
                self.builder.finish_node();
                true
            }
            Some(SyntaxKind::Minus) => match self.peek_nontrivia(1) {
                Some(SyntaxKind::DecimalMagnitude) => {
                    self.builder
                        .start_node(SyntaxKind::DecimalIntegerLiteral.into());
                    self.bump();
                    self.expect(
                        SyntaxKind::DecimalMagnitude,
                        ExpectedSyntax::DecimalMagnitude,
                    );
                    self.builder.finish_node();
                    true
                }
                Some(SyntaxKind::DecimalFloatingMagnitude) => {
                    self.builder
                        .start_node(SyntaxKind::DecimalFloatingLiteral.into());
                    self.bump();
                    self.expect(
                        SyntaxKind::DecimalFloatingMagnitude,
                        ExpectedSyntax::DecimalMagnitude,
                    );
                    self.builder.finish_node();
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn parse_trait_declaration(&mut self, exported: bool) {
        debug_assert!(self.at_contextual_ident(usize::from(exported), "trait"));
        self.builder.start_node(SyntaxKind::TraitDeclaration.into());
        if exported {
            self.expect(SyntaxKind::KwExport, ExpectedSyntax::Item);
        }
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Item);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        self.expect(SyntaxKind::Semicolon, ExpectedSyntax::Semicolon);
        self.builder.finish_node();
    }

    fn parse_trait_implementation(&mut self) {
        debug_assert!(self.at_contextual_ident(0, "impl"));
        self.builder
            .start_node(SyntaxKind::TraitImplementation.into());
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Item);
        self.parse_implementation_target();
        self.expect(SyntaxKind::Colon, ExpectedSyntax::Colon);
        self.parse_trait_reference();
        self.expect(SyntaxKind::Semicolon, ExpectedSyntax::Semicolon);
        self.builder.finish_node();
    }

    fn parse_implementation_target(&mut self) {
        self.builder
            .start_node(SyntaxKind::ImplementationTarget.into());
        if self.at(SyntaxKind::Ident) && self.peek_nontrivia(1) == Some(SyntaxKind::ColonColon) {
            self.parse_qualified_module_member();
        } else if self.current().is_some_and(is_reference_referent_type_start) {
            self.bump();
        } else {
            self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Type));
        }
        self.builder.finish_node();
    }

    fn parse_trait_reference(&mut self) {
        self.builder.start_node(SyntaxKind::TraitReference.into());
        if self.at(SyntaxKind::Ident) && self.peek_nontrivia(1) == Some(SyntaxKind::ColonColon) {
            self.parse_qualified_module_member();
        } else {
            self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        }
        self.builder.finish_node();
    }

    fn parse_record_definition(&mut self, exported: bool) {
        self.builder.start_node(SyntaxKind::RecordDefinition.into());
        if exported {
            self.expect(SyntaxKind::KwExport, ExpectedSyntax::Item);
        }
        self.expect(SyntaxKind::KwRecord, ExpectedSyntax::Item);
        self.eat(SyntaxKind::KwCopy);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);

        if !self.expect(SyntaxKind::LBrace, ExpectedSyntax::LeftBrace) {
            self.builder.finish_node();
            return;
        }

        self.bump_trivia();
        let mut missing_close = false;
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            let exported_field =
                self.at(SyntaxKind::KwExport) && self.peek_nontrivia(1) == Some(SyntaxKind::Ident);
            if self.at_any(TOP_LEVEL_STARTERS) && !exported_field {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::RightBrace));
                missing_close = true;
                break;
            }

            if self.at(SyntaxKind::Ident) || exported_field {
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
        self.eat(SyntaxKind::KwExport);
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
        if self.at(SyntaxKind::LBracket) {
            self.parse_generic_type_parameter_list();
        }
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

    fn parse_generic_type_parameter_list(&mut self) {
        debug_assert!(self.at(SyntaxKind::LBracket));
        self.builder
            .start_node(SyntaxKind::GenericTypeParameterList.into());
        self.bump();
        self.bump_trivia();

        if self.at(SyntaxKind::RBracket) {
            self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Identifier));
            self.bump();
            self.builder.finish_node();
            return;
        }

        let mut missing_close = false;
        while !self.at(SyntaxKind::RBracket) && self.current().is_some() {
            if self.at(SyntaxKind::LParen) || self.at_any(TOP_LEVEL_STARTERS) {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::RightBracket));
                missing_close = true;
                break;
            }

            if self.at(SyntaxKind::Ident) {
                self.builder
                    .start_node(SyntaxKind::GenericTypeParameter.into());
                self.bump();
                if self.at(SyntaxKind::Colon) {
                    self.parse_trait_requirement_clause();
                }
                self.builder.finish_node();

                if self.eat(SyntaxKind::Comma) {
                    self.bump_trivia();
                    continue;
                }
                if !self.at(SyntaxKind::RBracket) {
                    self.error_here(SyntaxErrorKind::Expected(
                        ExpectedSyntax::CommaOrRightBracket,
                    ));
                    self.recover_until(&[
                        SyntaxKind::Comma,
                        SyntaxKind::RBracket,
                        SyntaxKind::LParen,
                        SyntaxKind::KwImport,
                        SyntaxKind::KwExport,
                        SyntaxKind::KwFn,
                        SyntaxKind::KwRecord,
                    ]);
                    self.eat(SyntaxKind::Comma);
                }
            } else {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Identifier));
                self.recover_until(&[
                    SyntaxKind::Ident,
                    SyntaxKind::Comma,
                    SyntaxKind::RBracket,
                    SyntaxKind::LParen,
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
            self.expect(SyntaxKind::RBracket, ExpectedSyntax::RightBracket);
        }
        self.builder.finish_node();
    }

    fn parse_trait_requirement_clause(&mut self) {
        self.builder
            .start_node(SyntaxKind::TraitRequirementClause.into());
        self.expect(SyntaxKind::Colon, ExpectedSyntax::Colon);
        self.parse_trait_reference();
        while self.eat(SyntaxKind::Plus) {
            self.parse_trait_reference();
        }
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
                Some(SyntaxKind::KwLet) => self.parse_let_statement(),
                Some(SyntaxKind::KwFault) => self.parse_fault_statement(),
                Some(SyntaxKind::KwBreak) => self.parse_break_statement(),
                Some(SyntaxKind::KwContinue) => self.parse_continue_statement(),
                Some(SyntaxKind::KwReturn) => {
                    self.parse_return_statement();
                    returned = true;
                }
                Some(SyntaxKind::KwIf) => self.parse_if_or_refutable_record_selection_statement(),
                Some(SyntaxKind::KwWhile) => self.parse_while_statement(),
                Some(SyntaxKind::KwRaw) => self.parse_raw_assign_statement(),
                Some(SyntaxKind::KwUnsafe) => self.parse_unsafe_block_statement(),
                Some(SyntaxKind::Star) => self.parse_reference_assign_statement(),
                Some(SyntaxKind::Ident) => self.parse_identifier_statement(),
                Some(SyntaxKind::LBrace) => self.parse_block_statement(),
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

    fn parse_block_statement(&mut self) {
        self.parse_block_statement_with_else_boundary(false);
    }

    fn parse_block_statement_with_else_boundary(&mut self, stop_at_else: bool) {
        self.builder.start_node(SyntaxKind::BlockStatement.into());
        if !self.expect(SyntaxKind::LBrace, ExpectedSyntax::LeftBrace) {
            self.builder.finish_node();
            return;
        }

        self.bump_trivia();
        let mut returned = false;
        let mut missing_close = false;
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if stop_at_else && self.at(SyntaxKind::KwElse) {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::RightBrace));
                missing_close = true;
                break;
            }
            if self.at_any(TOP_LEVEL_STARTERS) {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::RightBrace));
                missing_close = true;
                break;
            }

            if returned {
                self.error_here(SyntaxErrorKind::UnexpectedAfterReturn);
                self.builder.start_node(SyntaxKind::ErrorNode.into());
                while !self.at(SyntaxKind::RBrace)
                    && !(stop_at_else && self.at(SyntaxKind::KwElse))
                    && !self.at_any(TOP_LEVEL_STARTERS)
                    && self.current().is_some()
                {
                    self.bump();
                    self.bump_trivia();
                }
                self.builder.finish_node();
                if (stop_at_else && self.at(SyntaxKind::KwElse)) || self.at_any(TOP_LEVEL_STARTERS)
                {
                    self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::RightBrace));
                    missing_close = true;
                }
                break;
            }

            match self.current() {
                Some(SyntaxKind::KwLet) => self.parse_let_statement(),
                Some(SyntaxKind::KwFault) => self.parse_fault_statement(),
                Some(SyntaxKind::KwBreak) => self.parse_break_statement(),
                Some(SyntaxKind::KwContinue) => self.parse_continue_statement(),
                Some(SyntaxKind::KwReturn) => {
                    self.parse_return_statement();
                    returned = true;
                }
                Some(SyntaxKind::KwIf) => self.parse_if_or_refutable_record_selection_statement(),
                Some(SyntaxKind::KwWhile) => self.parse_while_statement(),
                Some(SyntaxKind::KwRaw) => self.parse_raw_assign_statement(),
                Some(SyntaxKind::KwUnsafe) => self.parse_unsafe_block_statement(),
                Some(SyntaxKind::Star) => self.parse_reference_assign_statement(),
                Some(SyntaxKind::Ident) => self.parse_identifier_statement(),
                Some(SyntaxKind::LBrace) => self.parse_block_statement(),
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

    fn parse_unsafe_block_statement(&mut self) {
        self.builder
            .start_node(SyntaxKind::UnsafeBlockStatement.into());
        self.expect(SyntaxKind::KwUnsafe, ExpectedSyntax::Statement);
        self.parse_block_statement();
        self.builder.finish_node();
    }

    fn parse_if_or_refutable_record_selection_statement(&mut self) {
        debug_assert!(self.at(SyntaxKind::KwIf));
        if self.peek_nontrivia(1) == Some(SyntaxKind::KwLet) {
            self.parse_refutable_record_selection_statement();
        } else {
            self.parse_if_statement();
        }
    }

    fn parse_if_statement(&mut self) {
        self.builder.start_node(SyntaxKind::IfStatement.into());
        self.expect(SyntaxKind::KwIf, ExpectedSyntax::Statement);
        self.parse_conditional_value();
        self.parse_block_statement_with_else_boundary(true);
        if self.eat(SyntaxKind::KwElse) {
            self.parse_block_statement();
        }
        self.builder.finish_node();
    }

    fn parse_refutable_record_selection_statement(&mut self) {
        self.builder
            .start_node(SyntaxKind::RefutableRecordSelectionStatement.into());
        self.expect(SyntaxKind::KwIf, ExpectedSyntax::Statement);
        self.expect(SyntaxKind::KwLet, ExpectedSyntax::Statement);
        self.parse_refutable_record_pattern();
        self.expect(SyntaxKind::Eq, ExpectedSyntax::Equals);
        self.expect(SyntaxKind::LParen, ExpectedSyntax::LeftParen);
        self.parse_record_pattern_scrutinee();
        self.expect(SyntaxKind::RParen, ExpectedSyntax::RightParen);
        self.parse_block_statement_with_else_boundary(true);
        if self.eat(SyntaxKind::KwElse) {
            self.parse_block_statement();
        }
        self.builder.finish_node();
    }

    fn parse_while_statement(&mut self) {
        self.builder.start_node(SyntaxKind::WhileStatement.into());
        self.expect(SyntaxKind::KwWhile, ExpectedSyntax::Statement);
        self.parse_conditional_value();
        self.parse_block_statement();
        self.builder.finish_node();
    }

    fn parse_conditional_value(&mut self) {
        self.parse_logical_and_value(ValueContext::Conditional);
    }

    fn parse_let_statement(&mut self) {
        debug_assert!(self.at(SyntaxKind::KwLet));
        let unqualified_pattern = self.peek_nontrivia(1) == Some(SyntaxKind::Ident)
            && self.peek_nontrivia(2) == Some(SyntaxKind::LBrace);
        let qualified_pattern = self.peek_nontrivia(1) == Some(SyntaxKind::Ident)
            && self.peek_nontrivia(2) == Some(SyntaxKind::ColonColon)
            && self.peek_nontrivia(3) == Some(SyntaxKind::Ident)
            && self.peek_nontrivia(4) == Some(SyntaxKind::LBrace);
        if unqualified_pattern || qualified_pattern {
            self.parse_record_destructuring_declaration();
        } else {
            self.parse_local_declaration();
        }
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

    fn parse_record_destructuring_declaration(&mut self) {
        self.builder
            .start_node(SyntaxKind::RecordDestructuringDeclaration.into());
        self.expect(SyntaxKind::KwLet, ExpectedSyntax::Item);
        self.parse_record_pattern();
        self.expect(SyntaxKind::Eq, ExpectedSyntax::Equals);
        self.parse_record_pattern_scrutinee();
        self.expect(SyntaxKind::Semicolon, ExpectedSyntax::Semicolon);
        self.builder.finish_node();
    }

    fn parse_record_pattern(&mut self) {
        self.parse_record_pattern_in(RecordPatternContext::Irrefutable);
    }

    fn parse_refutable_record_pattern(&mut self) {
        self.parse_record_pattern_in(RecordPatternContext::Refutable);
    }

    fn parse_record_pattern_in(&mut self, context: RecordPatternContext) {
        let node_kind = match context {
            RecordPatternContext::Irrefutable => SyntaxKind::RecordPattern,
            RecordPatternContext::Refutable => SyntaxKind::RefutableRecordPattern,
        };
        self.builder.start_node(node_kind.into());
        if self.at(SyntaxKind::Ident) && self.peek_nontrivia(1) == Some(SyntaxKind::ColonColon) {
            self.parse_qualified_module_member();
        } else {
            self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        }
        if !self.expect(SyntaxKind::LBrace, ExpectedSyntax::LeftBrace) {
            self.builder.finish_node();
            return;
        }

        self.bump_trivia();
        let mut missing_close = false;
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if self.at(SyntaxKind::Eq)
                || self.at(SyntaxKind::Semicolon)
                || self.at(SyntaxKind::LBrace)
                || self.at(SyntaxKind::KwLet)
                || self.at(SyntaxKind::KwFault)
                || self.at(SyntaxKind::KwBreak)
                || self.at(SyntaxKind::KwContinue)
                || self.at(SyntaxKind::KwIf)
                || self.at(SyntaxKind::KwWhile)
                || self.at(SyntaxKind::KwRaw)
                || self.at(SyntaxKind::KwUnsafe)
                || self.at(SyntaxKind::KwElse)
                || self.at(SyntaxKind::KwReturn)
                || self.at_any(TOP_LEVEL_STARTERS)
            {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::RightBrace));
                missing_close = true;
                break;
            }

            if self.at(SyntaxKind::DotDot) {
                self.builder
                    .start_node(SyntaxKind::RecordPatternRest.into());
                self.bump();
                self.builder.finish_node();
                self.eat(SyntaxKind::Comma);
                self.bump_trivia();
                if !self.at(SyntaxKind::RBrace)
                    && self.current().is_some()
                    && !self.at_any(&[
                        SyntaxKind::Eq,
                        SyntaxKind::Semicolon,
                        SyntaxKind::LBrace,
                        SyntaxKind::KwLet,
                        SyntaxKind::KwFault,
                        SyntaxKind::KwBreak,
                        SyntaxKind::KwContinue,
                        SyntaxKind::KwIf,
                        SyntaxKind::KwWhile,
                        SyntaxKind::KwRaw,
                        SyntaxKind::KwUnsafe,
                        SyntaxKind::KwElse,
                        SyntaxKind::KwReturn,
                    ])
                    && !self.at_any(TOP_LEVEL_STARTERS)
                {
                    self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::RightBrace));
                    self.recover_until(&[
                        SyntaxKind::RBrace,
                        SyntaxKind::Eq,
                        SyntaxKind::Semicolon,
                        SyntaxKind::LBrace,
                        SyntaxKind::KwLet,
                        SyntaxKind::KwFault,
                        SyntaxKind::KwBreak,
                        SyntaxKind::KwContinue,
                        SyntaxKind::KwIf,
                        SyntaxKind::KwWhile,
                        SyntaxKind::KwRaw,
                        SyntaxKind::KwUnsafe,
                        SyntaxKind::KwElse,
                        SyntaxKind::KwReturn,
                        SyntaxKind::KwImport,
                        SyntaxKind::KwExport,
                        SyntaxKind::KwFn,
                        SyntaxKind::KwRecord,
                    ]);
                }
                continue;
            }

            if self.at(SyntaxKind::Ident) {
                self.parse_record_pattern_field_in(context);
                if self.eat(SyntaxKind::Comma) {
                    self.bump_trivia();
                    continue;
                }
                if !self.at(SyntaxKind::RBrace) {
                    self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::CommaOrRightBrace));
                    self.recover_until(&[
                        SyntaxKind::Ident,
                        SyntaxKind::DotDot,
                        SyntaxKind::Comma,
                        SyntaxKind::RBrace,
                        SyntaxKind::Eq,
                        SyntaxKind::Semicolon,
                        SyntaxKind::LBrace,
                        SyntaxKind::KwLet,
                        SyntaxKind::KwFault,
                        SyntaxKind::KwBreak,
                        SyntaxKind::KwContinue,
                        SyntaxKind::KwIf,
                        SyntaxKind::KwWhile,
                        SyntaxKind::KwRaw,
                        SyntaxKind::KwUnsafe,
                        SyntaxKind::KwElse,
                        SyntaxKind::KwReturn,
                        SyntaxKind::KwImport,
                        SyntaxKind::KwExport,
                        SyntaxKind::KwFn,
                        SyntaxKind::KwRecord,
                    ]);
                    self.eat(SyntaxKind::Comma);
                }
            } else {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Identifier));
                self.recover_until(&[
                    SyntaxKind::Ident,
                    SyntaxKind::DotDot,
                    SyntaxKind::Comma,
                    SyntaxKind::RBrace,
                    SyntaxKind::Eq,
                    SyntaxKind::Semicolon,
                    SyntaxKind::LBrace,
                    SyntaxKind::KwLet,
                    SyntaxKind::KwFault,
                    SyntaxKind::KwBreak,
                    SyntaxKind::KwContinue,
                    SyntaxKind::KwIf,
                    SyntaxKind::KwWhile,
                    SyntaxKind::KwRaw,
                    SyntaxKind::KwUnsafe,
                    SyntaxKind::KwElse,
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
            self.expect(SyntaxKind::RBrace, ExpectedSyntax::RightBrace);
        }
        self.builder.finish_node();
    }

    fn parse_record_pattern_field_in(&mut self, context: RecordPatternContext) {
        let node_kind = match context {
            RecordPatternContext::Irrefutable => SyntaxKind::RecordPatternField,
            RecordPatternContext::Refutable => SyntaxKind::RefutableRecordPatternField,
        };
        self.builder.start_node(node_kind.into());
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        self.expect(SyntaxKind::Colon, ExpectedSyntax::Colon);
        if self.at(SyntaxKind::Ident) {
            match self.peek_nontrivia(1) {
                Some(SyntaxKind::LBrace) => self.parse_record_pattern_in(context),
                Some(SyntaxKind::ColonColon)
                    if self.qualified_member_follower() == Some(SyntaxKind::LBrace) =>
                {
                    self.parse_record_pattern_in(context);
                }
                Some(SyntaxKind::EqEq) if matches!(context, RecordPatternContext::Refutable) => {
                    self.bump();
                    self.bump();
                    if !self.parse_refutable_record_literal_test() {
                        self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Value));
                        if self.current().is_some()
                            && !self.at_any(&[SyntaxKind::Comma, SyntaxKind::RBrace])
                        {
                            self.recover_one();
                        }
                    }
                }
                Some(SyntaxKind::Less) if matches!(context, RecordPatternContext::Refutable) => {
                    self.bump();
                    self.parse_refutable_record_strict_upper_bound_test();
                }
                _ => self.bump(),
            }
        } else if matches!(context, RecordPatternContext::Refutable) && self.at(SyntaxKind::Less) {
            self.parse_refutable_record_strict_upper_bound_test();
        } else if matches!(context, RecordPatternContext::Refutable)
            && self.parse_refutable_record_literal_test()
        {
        } else {
            let expected = if matches!(context, RecordPatternContext::Irrefutable) {
                ExpectedSyntax::Identifier
            } else {
                ExpectedSyntax::Value
            };
            self.error_here(SyntaxErrorKind::Expected(expected));
            if matches!(context, RecordPatternContext::Refutable)
                && self.current().is_some()
                && !self.at_any(&[SyntaxKind::Comma, SyntaxKind::RBrace])
            {
                self.recover_one();
            }
        }
        self.builder.finish_node();
    }

    fn parse_refutable_record_strict_upper_bound_test(&mut self) {
        debug_assert!(self.at(SyntaxKind::Less));
        self.bump();
        if !self.parse_refutable_record_integer_literal_test() {
            self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::DecimalMagnitude));
            if self.current().is_some() && !self.at_any(&[SyntaxKind::Comma, SyntaxKind::RBrace]) {
                self.recover_one();
            }
        }
    }

    fn parse_refutable_record_literal_test(&mut self) -> bool {
        match self.current() {
            Some(SyntaxKind::KwTrue | SyntaxKind::KwFalse) => {
                self.builder.start_node(SyntaxKind::BooleanLiteral.into());
                self.bump();
                self.builder.finish_node();
                true
            }
            _ => self.parse_refutable_record_integer_literal_test(),
        }
    }

    fn parse_refutable_record_integer_literal_test(&mut self) -> bool {
        match self.current() {
            Some(SyntaxKind::DecimalMagnitude) => {
                self.builder
                    .start_node(SyntaxKind::DecimalIntegerLiteral.into());
                self.bump();
                self.builder.finish_node();
                true
            }
            Some(SyntaxKind::Minus)
                if self.peek_nontrivia(1) == Some(SyntaxKind::DecimalMagnitude) =>
            {
                self.builder
                    .start_node(SyntaxKind::DecimalIntegerLiteral.into());
                self.bump();
                self.expect(
                    SyntaxKind::DecimalMagnitude,
                    ExpectedSyntax::DecimalMagnitude,
                );
                self.builder.finish_node();
                true
            }
            _ => false,
        }
    }

    fn parse_record_pattern_scrutinee(&mut self) {
        if !self.at(SyntaxKind::Ident) {
            self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Identifier));
            return;
        }

        match self.peek_nontrivia(1) {
            Some(SyntaxKind::LBrace) => self.parse_record_construction_or_field_value_use(),
            Some(SyntaxKind::LParen | SyntaxKind::LBracket) => {
                self.parse_direct_call_or_field_value_use();
            }
            Some(SyntaxKind::ColonColon) => match self.qualified_member_follower() {
                Some(SyntaxKind::LBrace) => self.parse_record_construction_or_field_value_use(),
                _ => self.parse_direct_call_or_field_value_use(),
            },
            Some(SyntaxKind::Dot) => self.parse_binding_field_value_use(),
            _ => self.bump(),
        }
    }

    fn parse_identifier_statement(&mut self) {
        match self.peek_nontrivia(1) {
            Some(SyntaxKind::Eq | SyntaxKind::Dot) => self.parse_assignment_statement(),
            Some(SyntaxKind::LParen | SyntaxKind::LBracket | SyntaxKind::ColonColon) => {
                self.parse_call_statement();
            }
            _ => {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Statement));
                self.recover_until(&[
                    SyntaxKind::Semicolon,
                    SyntaxKind::RBrace,
                    SyntaxKind::LBrace,
                    SyntaxKind::KwLet,
                    SyntaxKind::KwFault,
                    SyntaxKind::KwBreak,
                    SyntaxKind::KwContinue,
                    SyntaxKind::KwIf,
                    SyntaxKind::KwWhile,
                    SyntaxKind::KwRaw,
                    SyntaxKind::KwUnsafe,
                    SyntaxKind::KwElse,
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
        if self.at(SyntaxKind::Dot) {
            self.parse_field_selectors();
        }
        self.expect(SyntaxKind::Eq, ExpectedSyntax::Equals);
        self.parse_value();
        self.expect(SyntaxKind::Semicolon, ExpectedSyntax::Semicolon);
        self.builder.finish_node();
    }

    fn parse_reference_assign_statement(&mut self) {
        self.builder
            .start_node(SyntaxKind::ReferenceAssignStatement.into());
        self.expect(SyntaxKind::Star, ExpectedSyntax::Statement);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        self.expect(SyntaxKind::Eq, ExpectedSyntax::Equals);
        self.parse_value();
        self.expect(SyntaxKind::Semicolon, ExpectedSyntax::Semicolon);
        self.builder.finish_node();
    }

    fn parse_raw_assign_statement(&mut self) {
        self.builder
            .start_node(SyntaxKind::RawAssignStatement.into());
        self.expect(SyntaxKind::KwRaw, ExpectedSyntax::Statement);
        if self.at_contextual_ident(0, "assign") {
            self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        } else {
            self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Identifier));
            self.recover_one();
        }
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

    fn parse_fault_statement(&mut self) {
        self.builder.start_node(SyntaxKind::FaultStatement.into());
        self.expect(SyntaxKind::KwFault, ExpectedSyntax::Statement);
        self.expect(SyntaxKind::Semicolon, ExpectedSyntax::Semicolon);
        self.builder.finish_node();
    }

    fn parse_break_statement(&mut self) {
        self.builder.start_node(SyntaxKind::BreakStatement.into());
        self.expect(SyntaxKind::KwBreak, ExpectedSyntax::Statement);
        self.expect(SyntaxKind::Semicolon, ExpectedSyntax::Semicolon);
        self.builder.finish_node();
    }

    fn parse_continue_statement(&mut self) {
        self.builder
            .start_node(SyntaxKind::ContinueStatement.into());
        self.expect(SyntaxKind::KwContinue, ExpectedSyntax::Statement);
        self.expect(SyntaxKind::Semicolon, ExpectedSyntax::Semicolon);
        self.builder.finish_node();
    }

    fn parse_return_statement(&mut self) {
        self.builder.start_node(SyntaxKind::ReturnStatement.into());
        self.expect(SyntaxKind::KwReturn, ExpectedSyntax::Item);
        if self
            .current()
            .is_some_and(|kind| value_start_in(kind, ValueContext::Ordinary))
        {
            self.parse_value();
        }
        self.expect(SyntaxKind::Semicolon, ExpectedSyntax::Semicolon);
        self.builder.finish_node();
    }

    fn parse_type(&mut self) {
        self.builder.start_node(SyntaxKind::TypeRef.into());
        if self.eat(SyntaxKind::KwRaw) {
            self.parse_reference_referent_type();
        } else {
            if self.eat(SyntaxKind::Amp) {
                self.eat(SyntaxKind::KwMut);
            }
            self.parse_reference_referent_type();
        }
        self.builder.finish_node();
    }

    fn parse_reference_referent_type(&mut self) {
        if self.at(SyntaxKind::Ident) && self.peek_nontrivia(1) == Some(SyntaxKind::ColonColon) {
            self.parse_qualified_module_member();
        } else if self.current().is_some_and(is_reference_referent_type_start) {
            self.bump();
        } else {
            self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Type));
        }
    }

    fn parse_value(&mut self) {
        self.parse_logical_and_value(ValueContext::Ordinary);
    }

    fn parse_logical_and_value(&mut self, context: ValueContext) {
        let checkpoint = self.builder.checkpoint();
        self.parse_comparison_value(context);
        if self.at(SyntaxKind::AmpAmp) {
            self.builder
                .start_node_at(checkpoint, SyntaxKind::BooleanAndValue.into());
            self.bump();
            self.parse_comparison_value(context);
            self.builder.finish_node();
        }
    }

    fn parse_comparison_value(&mut self, context: ValueContext) {
        let checkpoint = self.builder.checkpoint();
        self.parse_or_value(context);
        if self.at(SyntaxKind::EqEq) || self.at(SyntaxKind::BangEq) || self.at(SyntaxKind::Less) {
            self.builder
                .start_node_at(checkpoint, SyntaxKind::BooleanEqualityValue.into());
            self.bump();
            self.parse_or_value(context);
            self.builder.finish_node();
        }
    }

    fn parse_or_value(&mut self, context: ValueContext) {
        let checkpoint = self.builder.checkpoint();
        self.parse_xor_value(context);
        if self.at(SyntaxKind::Pipe) {
            self.builder
                .start_node_at(checkpoint, SyntaxKind::IntegerOrValue.into());
            self.bump();
            self.parse_xor_value(context);
            self.builder.finish_node();
        }
    }

    fn parse_xor_value(&mut self, context: ValueContext) {
        let checkpoint = self.builder.checkpoint();
        self.parse_additive_value(context);
        if self.at(SyntaxKind::Caret) {
            self.builder
                .start_node_at(checkpoint, SyntaxKind::IntegerXorValue.into());
            self.bump();
            self.parse_additive_value(context);
            self.builder.finish_node();
        }
    }

    fn parse_additive_value(&mut self, context: ValueContext) {
        let checkpoint = self.builder.checkpoint();
        self.parse_multiplicative_value(context);
        let operation = match self.current() {
            Some(SyntaxKind::Plus) => Some(SyntaxKind::AddValue),
            Some(SyntaxKind::Minus) => Some(SyntaxKind::SubValue),
            _ => None,
        };
        if let Some(operation) = operation {
            self.builder.start_node_at(checkpoint, operation.into());
            self.bump();
            self.parse_multiplicative_value(context);
            self.builder.finish_node();
        }
    }

    fn parse_multiplicative_value(&mut self, context: ValueContext) {
        let checkpoint = self.builder.checkpoint();
        self.parse_value_in(context);
        if (self.at(SyntaxKind::Star) || self.at(SyntaxKind::Slash))
            && self
                .peek_nontrivia(1)
                .is_some_and(|kind| value_start_in(kind, context))
        {
            self.builder
                .start_node_at(checkpoint, SyntaxKind::MulValue.into());
            self.bump();
            self.parse_value_in(context);
            self.builder.finish_node();
        }
    }

    fn parse_value_in(&mut self, context: ValueContext) {
        match self.current() {
            Some(SyntaxKind::Amp) if matches!(context, ValueContext::Ordinary) => {
                self.parse_safe_reference_value();
            }
            Some(SyntaxKind::Star) if matches!(context, ValueContext::Ordinary) => {
                self.parse_reference_dereference_value();
            }
            Some(SyntaxKind::KwRaw) if matches!(context, ValueContext::Ordinary) => {
                self.parse_raw_value();
            }
            Some(SyntaxKind::At)
                if matches!(context, ValueContext::Ordinary) && self.at_fast_selector_start() =>
            {
                self.parse_numeric_contract_selected_value();
            }
            Some(SyntaxKind::Bang) => self.parse_boolean_not_value(context),
            Some(SyntaxKind::Tilde) => self.parse_integer_complement_value(context),
            Some(SyntaxKind::LParen) => self.parse_grouped_value(context),
            Some(SyntaxKind::Ident) => match context {
                ValueContext::Ordinary => match self.peek_nontrivia(1) {
                    Some(SyntaxKind::LBrace) => self.parse_record_construction_or_field_value_use(),
                    Some(SyntaxKind::LParen | SyntaxKind::LBracket) => {
                        self.parse_direct_call_or_field_value_use();
                    }
                    Some(SyntaxKind::ColonColon) => match self.qualified_member_follower() {
                        Some(SyntaxKind::LBrace) => {
                            self.parse_record_construction_or_field_value_use();
                        }
                        Some(SyntaxKind::LParen | SyntaxKind::LBracket) => {
                            self.parse_direct_call_or_field_value_use();
                        }
                        _ => self.parse_qualified_module_member(),
                    },
                    Some(SyntaxKind::Dot) => self.parse_binding_field_value_use(),
                    _ => {
                        self.builder.start_node(SyntaxKind::IdentifierUse.into());
                        self.bump();
                        self.builder.finish_node();
                    }
                },
                ValueContext::Conditional => match self.peek_nontrivia(1) {
                    Some(SyntaxKind::LParen | SyntaxKind::LBracket) => {
                        self.parse_direct_call_or_field_value_use();
                    }
                    Some(SyntaxKind::ColonColon) => match self.qualified_member_follower() {
                        Some(SyntaxKind::LParen | SyntaxKind::LBracket) => {
                            self.parse_direct_call_or_field_value_use();
                        }
                        Some(SyntaxKind::LBrace)
                            if self.record_construction_followed_by_selector() =>
                        {
                            self.parse_record_construction_or_field_value_use();
                        }
                        _ => self.parse_qualified_module_member(),
                    },
                    Some(SyntaxKind::Dot) => self.parse_binding_field_value_use(),
                    Some(SyntaxKind::LBrace) if self.record_construction_followed_by_selector() => {
                        self.parse_record_construction_or_field_value_use();
                    }
                    _ => {
                        self.builder.start_node(SyntaxKind::IdentifierUse.into());
                        self.bump();
                        self.builder.finish_node();
                    }
                },
            },
            Some(SyntaxKind::KwTrue | SyntaxKind::KwFalse) => {
                self.builder.start_node(SyntaxKind::BooleanLiteral.into());
                self.bump();
                self.builder.finish_node();
            }
            Some(SyntaxKind::DecimalMagnitude) => {
                self.builder
                    .start_node(SyntaxKind::DecimalIntegerLiteral.into());
                self.bump();
                self.builder.finish_node();
            }
            Some(SyntaxKind::DecimalFloatingMagnitude) => {
                self.builder
                    .start_node(SyntaxKind::DecimalFloatingLiteral.into());
                self.bump();
                self.builder.finish_node();
            }
            Some(SyntaxKind::Minus) => match self.peek_nontrivia(1) {
                Some(SyntaxKind::DecimalMagnitude) => {
                    self.builder
                        .start_node(SyntaxKind::DecimalIntegerLiteral.into());
                    self.bump();
                    self.expect(
                        SyntaxKind::DecimalMagnitude,
                        ExpectedSyntax::DecimalMagnitude,
                    );
                    self.builder.finish_node();
                }
                Some(SyntaxKind::DecimalFloatingMagnitude) => {
                    self.builder
                        .start_node(SyntaxKind::DecimalFloatingLiteral.into());
                    self.bump();
                    self.expect(
                        SyntaxKind::DecimalFloatingMagnitude,
                        ExpectedSyntax::DecimalMagnitude,
                    );
                    self.builder.finish_node();
                }
                _ => self.parse_integer_neg_value(context),
            },
            _ => {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Value));
                let is_boundary = match context {
                    ValueContext::Ordinary => {
                        self.at_any(&[
                            SyntaxKind::Comma,
                            SyntaxKind::RParen,
                            SyntaxKind::Semicolon,
                            SyntaxKind::RBrace,
                            SyntaxKind::LBrace,
                            SyntaxKind::KwLet,
                            SyntaxKind::KwFault,
                            SyntaxKind::KwBreak,
                            SyntaxKind::KwContinue,
                            SyntaxKind::KwIf,
                            SyntaxKind::KwWhile,
                            SyntaxKind::KwUnsafe,
                            SyntaxKind::KwElse,
                            SyntaxKind::KwReturn,
                        ]) || self.at_any(TOP_LEVEL_STARTERS)
                    }
                    ValueContext::Conditional => {
                        self.at_any(&[
                            SyntaxKind::LBrace,
                            SyntaxKind::RBrace,
                            SyntaxKind::KwElse,
                            SyntaxKind::KwLet,
                            SyntaxKind::KwFault,
                            SyntaxKind::KwBreak,
                            SyntaxKind::KwContinue,
                            SyntaxKind::KwIf,
                            SyntaxKind::KwWhile,
                            SyntaxKind::KwUnsafe,
                            SyntaxKind::KwReturn,
                        ]) || self.at_any(TOP_LEVEL_STARTERS)
                    }
                };
                if self.current().is_some() && !is_boundary {
                    self.recover_one();
                }
            }
        }
    }

    fn parse_raw_value(&mut self) {
        debug_assert!(self.at(SyntaxKind::KwRaw));
        match self.peek_nontrivia(1) {
            Some(SyntaxKind::Amp) => self.parse_raw_address_of_value(),
            Some(SyntaxKind::Ident) if self.at_contextual_ident(1, "move") => {
                self.parse_raw_move_value();
            }
            _ => {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Value));
                self.recover_one();
            }
        }
    }

    fn parse_raw_address_of_value(&mut self) {
        self.builder
            .start_node(SyntaxKind::RawAddressOfValue.into());
        self.expect(SyntaxKind::KwRaw, ExpectedSyntax::Value);
        self.expect(SyntaxKind::Amp, ExpectedSyntax::Value);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        self.builder.finish_node();
    }

    fn parse_raw_move_value(&mut self) {
        self.builder.start_node(SyntaxKind::RawMoveValue.into());
        self.expect(SyntaxKind::KwRaw, ExpectedSyntax::Value);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        self.builder.finish_node();
    }

    fn parse_safe_reference_value(&mut self) {
        self.builder
            .start_node(SyntaxKind::SafeReferenceValue.into());
        self.expect(SyntaxKind::Amp, ExpectedSyntax::Value);
        self.eat(SyntaxKind::KwMut);
        self.eat(SyntaxKind::Star);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        if self.at(SyntaxKind::Dot) {
            self.parse_field_selectors();
        }
        self.builder.finish_node();
    }

    fn parse_reference_dereference_value(&mut self) {
        self.builder
            .start_node(SyntaxKind::ReferenceDereferenceValue.into());
        self.expect(SyntaxKind::Star, ExpectedSyntax::Value);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        self.builder.finish_node();
    }

    fn parse_numeric_contract_selected_value(&mut self) {
        debug_assert!(self.at_fast_selector_start());
        self.builder
            .start_node(SyntaxKind::NumericContractSelectedValue.into());
        self.expect(SyntaxKind::At, ExpectedSyntax::Value);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        if !self.expect(SyntaxKind::LParen, ExpectedSyntax::LeftParen) {
            self.builder.finish_node();
            return;
        }
        if self.group_inner_value_is_missing() {
            self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Value));
        } else {
            self.parse_value();
        }
        self.expect(SyntaxKind::RParen, ExpectedSyntax::RightParen);
        self.builder.finish_node();
    }

    fn parse_boolean_not_value(&mut self, context: ValueContext) {
        self.builder.start_node(SyntaxKind::BooleanNotValue.into());
        self.expect(SyntaxKind::Bang, ExpectedSyntax::Value);
        self.parse_value_in(context);
        self.builder.finish_node();
    }

    fn parse_integer_neg_value(&mut self, context: ValueContext) {
        self.builder.start_node(SyntaxKind::IntegerNegValue.into());
        self.expect(SyntaxKind::Minus, ExpectedSyntax::Value);
        self.parse_value_in(context);
        self.builder.finish_node();
    }

    fn parse_integer_complement_value(&mut self, context: ValueContext) {
        self.builder
            .start_node(SyntaxKind::IntegerComplementValue.into());
        self.expect(SyntaxKind::Tilde, ExpectedSyntax::Value);
        self.parse_value_in(context);
        self.builder.finish_node();
    }

    fn parse_grouped_value(&mut self, context: ValueContext) {
        self.builder.start_node(SyntaxKind::GroupedValue.into());
        self.expect(SyntaxKind::LParen, ExpectedSyntax::LeftParen);
        if self.group_inner_value_is_missing() {
            self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Value));
        } else {
            self.parse_logical_and_value(context);
        }
        self.expect(SyntaxKind::RParen, ExpectedSyntax::RightParen);
        self.builder.finish_node();
    }

    fn group_inner_value_is_missing(&self) -> bool {
        self.current().is_none()
            || self.at_any(&[
                SyntaxKind::RParen,
                SyntaxKind::Comma,
                SyntaxKind::Semicolon,
                SyntaxKind::RBrace,
                SyntaxKind::LBrace,
                SyntaxKind::KwLet,
                SyntaxKind::KwFault,
                SyntaxKind::KwBreak,
                SyntaxKind::KwContinue,
                SyntaxKind::KwIf,
                SyntaxKind::KwWhile,
                SyntaxKind::KwUnsafe,
                SyntaxKind::KwElse,
                SyntaxKind::KwReturn,
            ])
            || self.at_any(TOP_LEVEL_STARTERS)
    }

    fn parse_binding_field_value_use(&mut self) {
        self.builder.start_node(SyntaxKind::FieldValueUse.into());
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        self.parse_field_selectors();
        self.builder.finish_node();
    }

    fn parse_direct_call_or_field_value_use(&mut self) {
        let checkpoint = self.builder.checkpoint();
        self.parse_direct_call();
        if self.at(SyntaxKind::Dot) {
            self.builder
                .start_node_at(checkpoint, SyntaxKind::FieldValueUse.into());
            self.parse_field_selectors();
            self.builder.finish_node();
        }
    }

    fn parse_record_construction_or_field_value_use(&mut self) {
        let checkpoint = self.builder.checkpoint();
        self.parse_record_construction();
        if self.at(SyntaxKind::Dot) {
            self.builder
                .start_node_at(checkpoint, SyntaxKind::FieldValueUse.into());
            self.parse_field_selectors();
            self.builder.finish_node();
        }
    }

    fn parse_field_selectors(&mut self) {
        debug_assert!(self.at(SyntaxKind::Dot));
        while self.eat(SyntaxKind::Dot) {
            self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        }
    }

    fn qualified_member_follower(&self) -> Option<SyntaxKind> {
        (self.peek_nontrivia(1) == Some(SyntaxKind::ColonColon)
            && self.peek_nontrivia(2) == Some(SyntaxKind::Ident))
        .then(|| self.peek_nontrivia(3))
        .flatten()
    }

    fn record_construction_followed_by_selector(&self) -> bool {
        let mut tokens = self.tokens[self.position..]
            .iter()
            .filter(|token| !token.kind.is_trivia())
            .map(|token| token.kind);
        if tokens.next() != Some(SyntaxKind::Ident) {
            return false;
        }
        match tokens.next() {
            Some(SyntaxKind::LBrace) => {}
            Some(SyntaxKind::ColonColon) => {
                if tokens.next() != Some(SyntaxKind::Ident)
                    || tokens.next() != Some(SyntaxKind::LBrace)
                {
                    return false;
                }
            }
            _ => return false,
        }

        let mut depth = 1_usize;
        while let Some(kind) = tokens.next() {
            match kind {
                SyntaxKind::LBrace => depth += 1,
                SyntaxKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        return tokens.next() == Some(SyntaxKind::Dot);
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn parse_record_construction(&mut self) {
        self.builder
            .start_node(SyntaxKind::RecordConstruction.into());
        if self.at(SyntaxKind::Ident) && self.peek_nontrivia(1) == Some(SyntaxKind::ColonColon) {
            self.parse_qualified_module_member();
        } else {
            self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        }
        if !self.expect(SyntaxKind::LBrace, ExpectedSyntax::LeftBrace) {
            self.builder.finish_node();
            return;
        }

        self.bump_trivia();
        let mut missing_close = false;
        while !self.at(SyntaxKind::RBrace) && self.current().is_some() {
            if self.at(SyntaxKind::RParen)
                || self.at(SyntaxKind::Semicolon)
                || self.at(SyntaxKind::LBrace)
                || self.at(SyntaxKind::KwLet)
                || self.at(SyntaxKind::KwFault)
                || self.at(SyntaxKind::KwBreak)
                || self.at(SyntaxKind::KwContinue)
                || self.at(SyntaxKind::KwIf)
                || self.at(SyntaxKind::KwWhile)
                || self.at(SyntaxKind::KwRaw)
                || self.at(SyntaxKind::KwUnsafe)
                || self.at(SyntaxKind::KwElse)
                || self.at(SyntaxKind::KwReturn)
                || self.at_any(TOP_LEVEL_STARTERS)
            {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::RightBrace));
                missing_close = true;
                break;
            }

            if self.at(SyntaxKind::Ident) {
                self.builder
                    .start_node(SyntaxKind::RecordInitializer.into());
                self.bump();
                self.expect(SyntaxKind::Colon, ExpectedSyntax::Colon);
                self.parse_value();
                self.builder.finish_node();

                if self.eat(SyntaxKind::Comma) {
                    self.bump_trivia();
                    continue;
                }
                if !self.at(SyntaxKind::RBrace) {
                    self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::CommaOrRightBrace));
                    self.recover_until(&[
                        SyntaxKind::Ident,
                        SyntaxKind::Comma,
                        SyntaxKind::RBrace,
                        SyntaxKind::RParen,
                        SyntaxKind::Semicolon,
                        SyntaxKind::LBrace,
                        SyntaxKind::KwLet,
                        SyntaxKind::KwFault,
                        SyntaxKind::KwBreak,
                        SyntaxKind::KwContinue,
                        SyntaxKind::KwIf,
                        SyntaxKind::KwWhile,
                        SyntaxKind::KwRaw,
                        SyntaxKind::KwUnsafe,
                        SyntaxKind::KwElse,
                        SyntaxKind::KwReturn,
                        SyntaxKind::KwImport,
                        SyntaxKind::KwExport,
                        SyntaxKind::KwFn,
                        SyntaxKind::KwRecord,
                    ]);
                    self.eat(SyntaxKind::Comma);
                }
            } else {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Identifier));
                self.recover_until(&[
                    SyntaxKind::Ident,
                    SyntaxKind::Comma,
                    SyntaxKind::RBrace,
                    SyntaxKind::RParen,
                    SyntaxKind::Semicolon,
                    SyntaxKind::LBrace,
                    SyntaxKind::KwLet,
                    SyntaxKind::KwFault,
                    SyntaxKind::KwBreak,
                    SyntaxKind::KwContinue,
                    SyntaxKind::KwIf,
                    SyntaxKind::KwWhile,
                    SyntaxKind::KwRaw,
                    SyntaxKind::KwUnsafe,
                    SyntaxKind::KwElse,
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
            self.expect(SyntaxKind::RBrace, ExpectedSyntax::RightBrace);
        }
        self.builder.finish_node();
    }

    fn parse_direct_call(&mut self) {
        self.builder.start_node(SyntaxKind::DirectCall.into());
        if self.at(SyntaxKind::Ident) && self.peek_nontrivia(1) == Some(SyntaxKind::ColonColon) {
            self.parse_qualified_module_member();
        } else {
            self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        }

        if self.at(SyntaxKind::LBracket) {
            self.parse_generic_type_argument_list();
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
                || self.at(SyntaxKind::LBrace)
                || self.at(SyntaxKind::KwLet)
                || self.at(SyntaxKind::KwFault)
                || self.at(SyntaxKind::KwBreak)
                || self.at(SyntaxKind::KwContinue)
                || self.at(SyntaxKind::KwIf)
                || self.at(SyntaxKind::KwWhile)
                || self.at(SyntaxKind::KwUnsafe)
                || self.at(SyntaxKind::KwElse)
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
                    SyntaxKind::LBrace,
                    SyntaxKind::KwLet,
                    SyntaxKind::KwFault,
                    SyntaxKind::KwBreak,
                    SyntaxKind::KwContinue,
                    SyntaxKind::KwIf,
                    SyntaxKind::KwWhile,
                    SyntaxKind::KwRaw,
                    SyntaxKind::KwUnsafe,
                    SyntaxKind::KwElse,
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

    fn parse_generic_type_argument_list(&mut self) {
        debug_assert!(self.at(SyntaxKind::LBracket));
        self.builder
            .start_node(SyntaxKind::GenericTypeArgumentList.into());
        self.bump();
        self.bump_trivia();

        if self.at(SyntaxKind::RBracket) {
            self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Type));
            self.bump();
            self.builder.finish_node();
            return;
        }

        let mut missing_close = false;
        while !self.at(SyntaxKind::RBracket) && self.current().is_some() {
            if self.at(SyntaxKind::LParen)
                || self.at(SyntaxKind::Semicolon)
                || self.at(SyntaxKind::RParen)
                || self.at(SyntaxKind::RBrace)
                || self.at(SyntaxKind::LBrace)
                || self.at_any(TOP_LEVEL_STARTERS)
            {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::RightBracket));
                missing_close = true;
                break;
            }

            if self.current().is_some_and(is_generic_type_argument_start) {
                self.builder
                    .start_node(SyntaxKind::GenericTypeArgument.into());
                if self.at(SyntaxKind::Ident)
                    && self.peek_nontrivia(1) == Some(SyntaxKind::ColonColon)
                {
                    self.parse_qualified_module_member();
                } else {
                    self.bump();
                }
                self.builder.finish_node();

                if self.eat(SyntaxKind::Comma) {
                    self.bump_trivia();
                    continue;
                }
                if !self.at(SyntaxKind::RBracket) {
                    self.error_here(SyntaxErrorKind::Expected(
                        ExpectedSyntax::CommaOrRightBracket,
                    ));
                    self.recover_until(&[
                        SyntaxKind::Comma,
                        SyntaxKind::RBracket,
                        SyntaxKind::LParen,
                        SyntaxKind::Semicolon,
                        SyntaxKind::RParen,
                        SyntaxKind::RBrace,
                        SyntaxKind::LBrace,
                        SyntaxKind::KwImport,
                        SyntaxKind::KwExport,
                        SyntaxKind::KwFn,
                        SyntaxKind::KwRecord,
                    ]);
                    self.eat(SyntaxKind::Comma);
                }
            } else {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Type));
                self.recover_until(&[
                    SyntaxKind::Comma,
                    SyntaxKind::RBracket,
                    SyntaxKind::LParen,
                    SyntaxKind::Semicolon,
                    SyntaxKind::RParen,
                    SyntaxKind::RBrace,
                    SyntaxKind::LBrace,
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
            self.expect(SyntaxKind::RBracket, ExpectedSyntax::RightBracket);
        }
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

    fn peek_nontrivia_token(&self, index: usize) -> Option<LexToken> {
        self.tokens[self.position..]
            .iter()
            .filter(|token| !token.kind.is_trivia())
            .nth(index)
            .copied()
    }

    fn peek_nontrivia(&self, index: usize) -> Option<SyntaxKind> {
        self.peek_nontrivia_token(index).map(|token| token.kind)
    }

    fn at_contextual_ident(&self, index: usize, expected: &str) -> bool {
        let Some(token) = self.peek_nontrivia_token(index) else {
            return false;
        };
        token.kind == SyntaxKind::Ident
            && identifier_key(&self.source[token.start..token.end]).as_deref() == Some(expected)
    }

    fn at_fast_selector_start(&self) -> bool {
        self.current() == Some(SyntaxKind::At) && self.at_contextual_ident(1, "fast")
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

const fn is_intrinsic_type_start(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::TyBool
            | SyntaxKind::TyI8
            | SyntaxKind::TyI16
            | SyntaxKind::TyI32
            | SyntaxKind::TyI64
            | SyntaxKind::TyU8
            | SyntaxKind::TyU16
            | SyntaxKind::TyU32
            | SyntaxKind::TyU64
            | SyntaxKind::TyF16
            | SyntaxKind::TyF32
            | SyntaxKind::TyF64
    )
}

const fn is_reference_referent_type_start(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Ident
            | SyntaxKind::TyBool
            | SyntaxKind::TyI8
            | SyntaxKind::TyI16
            | SyntaxKind::TyI32
            | SyntaxKind::TyI64
            | SyntaxKind::TyU8
            | SyntaxKind::TyU16
            | SyntaxKind::TyU32
            | SyntaxKind::TyU64
            | SyntaxKind::TyF16
            | SyntaxKind::TyF32
            | SyntaxKind::TyF64
    )
}

const fn is_generic_type_argument_start(kind: SyntaxKind) -> bool {
    is_reference_referent_type_start(kind)
}

const fn value_start_in(kind: SyntaxKind, context: ValueContext) -> bool {
    kind.is_value_start()
        || (matches!(context, ValueContext::Ordinary)
            && matches!(kind, SyntaxKind::Amp | SyntaxKind::Star | SyntaxKind::KwRaw))
}
