//! Parser implementation for Aurora.

use crate::error::ParseError;
use aurora_ast::{BinaryOp, Expr, ExprKind, Ident, Literal, Program, Stmt, StmtKind};
use aurora_lexer::{Token, TokenKind};

/// Precedence levels for Pratt parsing binary expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    Lowest = 0,
    Pipe = 1,       // |>
    LogicalOr = 2,  // ||
    LogicalAnd = 3, // &&
    Comparison = 4, // ==, !=, <, <=, >, >=
    Bitwise = 5,    // |, ^, &
    Term = 6,       // +, -
    Factor = 7,     // *, /, %
    Unary = 8,      // -, !, *
    Call = 9,       // (), []
}

impl Precedence {
    pub fn for_token(kind: &TokenKind) -> Self {
        match kind {
            TokenKind::PipeGt => Self::Pipe,
            TokenKind::PipePipe => Self::LogicalOr,
            TokenKind::AmpAmp => Self::LogicalAnd,
            TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::Lt
            | TokenKind::LtEq
            | TokenKind::Gt
            | TokenKind::GtEq => Self::Comparison,
            TokenKind::Pipe | TokenKind::Ampersand | TokenKind::Caret => Self::Bitwise,
            TokenKind::Plus | TokenKind::Minus => Self::Term,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Self::Factor,
            TokenKind::OpenParen => Self::Call,
            _ => Self::Lowest,
        }
    }
}

/// Recursive descent and Pratt parser for Aurora syntax.
#[derive(Debug)]
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    /// Parses an entire program.
    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut program = Program::default();
        while !self.is_at_end() {
            let stmt = self.parse_statement()?;
            program.statements.push(stmt);
        }
        Ok(program)
    }

    pub fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        if self.check(&TokenKind::Let) {
            self.parse_let_statement()
        } else if self.check(&TokenKind::Return) {
            self.parse_return_statement()
        } else {
            let expr = self.parse_expression(Precedence::Lowest)?;
            // Optional trailing semicolon
            if self.check(&TokenKind::Semicolon) {
                self.advance();
            }
            let span = expr.span;
            Ok(Stmt::new(StmtKind::Expr(expr), span))
        }
    }

    fn parse_let_statement(&mut self) -> Result<Stmt, ParseError> {
        let let_token = self.consume(&TokenKind::Let, "expected 'let'")?;
        let is_mutable = if self.check(&TokenKind::Mut) {
            self.advance();
            true
        } else {
            false
        };

        let name = self.parse_ident()?;
        let type_annotation = if self.match_token(&TokenKind::Colon) {
            let type_ident = self.parse_ident()?;
            Some(type_ident.name)
        } else {
            None
        };

        let initializer = if self.match_token(&TokenKind::Equal) {
            Some(self.parse_expression(Precedence::Lowest)?)
        } else {
            None
        };

        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }

        let end_span = initializer.as_ref().map_or(name.span, |i| i.span);
        let span = let_token.span.merge(end_span);

        Ok(Stmt::new(
            StmtKind::Let {
                name,
                type_annotation,
                initializer,
                is_mutable,
            },
            span,
        ))
    }

    fn parse_return_statement(&mut self) -> Result<Stmt, ParseError> {
        let ret = self.consume(&TokenKind::Return, "expected 'return'")?;
        let value = if !self.check(&TokenKind::Semicolon) && !self.is_at_end() {
            Some(self.parse_expression(Precedence::Lowest)?)
        } else {
            None
        };

        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }

        let span = value.as_ref().map_or(ret.span, |v| ret.span.merge(v.span));
        Ok(Stmt::new(StmtKind::Return(value), span))
    }

    /// Pratt parser for expressions.
    pub fn parse_expression(&mut self, precedence: Precedence) -> Result<Expr, ParseError> {
        let mut left = self.parse_prefix()?;

        while !self.is_at_end() {
            let next_prec = Precedence::for_token(&self.peek().kind);
            if precedence >= next_prec {
                break;
            }

            left = self.parse_infix(left, next_prec)?;
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        let token = self.advance();
        match token.kind {
            TokenKind::Int { value, .. } => Ok(Expr::new(
                ExprKind::Literal(Literal::Int(value)),
                token.span,
            )),
            TokenKind::Float { value, .. } => Ok(Expr::new(
                ExprKind::Literal(Literal::Float(value)),
                token.span,
            )),
            TokenKind::Str(s) | TokenKind::RawStr(s) => {
                Ok(Expr::new(ExprKind::Literal(Literal::String(s)), token.span))
            }
            TokenKind::Bool(b) => Ok(Expr::new(ExprKind::Literal(Literal::Bool(b)), token.span)),
            TokenKind::Nil => Ok(Expr::new(ExprKind::Literal(Literal::Nil), token.span)),
            TokenKind::Ident(name) => Ok(Expr::new(
                ExprKind::Variable(Ident::new(name, token.span)),
                token.span,
            )),
            TokenKind::OpenParen => {
                let expr = self.parse_expression(Precedence::Lowest)?;
                let close = self.consume(&TokenKind::CloseParen, "expected ')'")?;
                Ok(Expr::new(expr.kind, token.span.merge(close.span)))
            }
            other => Err(ParseError::UnexpectedToken {
                expected: "expression".into(),
                found: other.to_string(),
                span: token.span,
            }),
        }
    }

    fn parse_infix(&mut self, left: Expr, prec: Precedence) -> Result<Expr, ParseError> {
        let op_token = self.advance();
        let op = match op_token.kind {
            TokenKind::Plus => BinaryOp::Add,
            TokenKind::Minus => BinaryOp::Sub,
            TokenKind::Star => BinaryOp::Mul,
            TokenKind::Slash => BinaryOp::Div,
            TokenKind::Percent => BinaryOp::Mod,
            TokenKind::EqEq => BinaryOp::Eq,
            TokenKind::BangEq => BinaryOp::NotEq,
            TokenKind::Lt => BinaryOp::Lt,
            TokenKind::LtEq => BinaryOp::LtEq,
            TokenKind::Gt => BinaryOp::Gt,
            TokenKind::GtEq => BinaryOp::GtEq,
            TokenKind::AmpAmp => BinaryOp::And,
            TokenKind::PipePipe => BinaryOp::Or,
            TokenKind::PipeGt => BinaryOp::Pipe,
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "binary operator".into(),
                    found: op_token.kind.to_string(),
                    span: op_token.span,
                })
            }
        };

        let right = self.parse_expression(prec)?;
        let span = left.span.merge(right.span);

        Ok(Expr::new(
            ExprKind::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            },
            span,
        ))
    }

    fn parse_ident(&mut self) -> Result<Ident, ParseError> {
        let tok = self.advance();
        if let TokenKind::Ident(name) = tok.kind {
            Ok(Ident::new(name, tok.span))
        } else {
            Err(ParseError::UnexpectedToken {
                expected: "identifier".into(),
                found: tok.kind.to_string(),
                span: tok.span,
            })
        }
    }

    fn check(&self, expected: &TokenKind) -> bool {
        if self.is_at_end() {
            return false;
        }
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(expected)
    }

    fn match_token(&mut self, expected: &TokenKind) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, expected: &TokenKind, msg: &str) -> Result<Token, ParseError> {
        if self.check(expected) {
            Ok(self.advance())
        } else {
            let peek = self.peek();
            Err(ParseError::UnexpectedToken {
                expected: msg.into(),
                found: peek.kind.to_string(),
                span: peek.span,
            })
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.position += 1;
        }
        self.tokens[self.position - 1].clone()
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len() || self.peek().kind == TokenKind::Eof
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aurora_lexer::Lexer;

    #[test]
    fn test_parse_let_and_binary_expr() {
        let code = "let total = 10 + 20 * 3";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_statement().unwrap();

        match stmt.kind {
            StmtKind::Let {
                name,
                initializer: Some(expr),
                ..
            } => {
                assert_eq!(name.name, "total");
                // Due to precedence, 20 * 3 binds tighter than 10 + ...
                if let ExprKind::Binary { op, .. } = expr.kind {
                    assert_eq!(op, BinaryOp::Add);
                } else {
                    panic!("Expected binary add");
                }
            }
            _ => panic!("Expected let statement"),
        }
    }
}
