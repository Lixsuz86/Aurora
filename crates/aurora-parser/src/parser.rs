//! Comprehensive Pratt and Recursive-Descent Parser for Aurora.

use crate::error::ParseError;
use aurora_ast::{
    AssignOp, BinaryOp, Block, EnumDecl, EnumVariant, EnumVariantKind, Expr, ExprKind,
    FunctionDecl, Ident, ImplDecl, Item, ItemKind, Literal, MatchArm, Param, Pattern, PatternKind,
    Program, Span, Stmt, StmtKind, StructDecl, StructField, TraitDecl, TypeAliasDecl, TypeExpr,
    TypeExprKind, UnaryOp, UseDecl,
};
use aurora_lexer::{Token, TokenKind};

/// Precedence levels for Pratt parsing binary and postfix expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    Lowest = 0,
    Assignment = 1, // =, +=, -=, *=, /=, %=, &=, |=, ^=, <<=, >>=
    Pipeline = 2,   // |>
    LogicalOr = 3,  // ||, ??
    LogicalAnd = 4, // &&
    BitwiseOr = 5,  // |
    BitwiseXor = 6, // ^
    BitwiseAnd = 7, // &
    Equality = 8,   // ==, !=
    Comparison = 9, // <, <=, >, >=
    Shift = 10,     // <<, >>
    Range = 11,     // .., ..=
    Term = 12,      // +, -
    Factor = 13,    // *, /, %
    Power = 14,     // **
    Cast = 15,      // as
    Unary = 16,     // prefix -, !, ~, *, &, &mut
    Call = 17,      // (), [], ., ?
}

impl Precedence {
    #[must_use]
    pub fn for_token(kind: &TokenKind) -> Self {
        match kind {
            TokenKind::Equal
            | TokenKind::PlusEq
            | TokenKind::MinusEq
            | TokenKind::StarEq
            | TokenKind::SlashEq
            | TokenKind::PercentEq
            | TokenKind::AmpEq
            | TokenKind::PipeEq
            | TokenKind::CaretEq
            | TokenKind::ShlEq
            | TokenKind::ShrEq => Self::Assignment,
            TokenKind::PipeGt => Self::Pipeline,
            TokenKind::PipePipe | TokenKind::QuestionQuestion => Self::LogicalOr,
            TokenKind::AmpAmp => Self::LogicalAnd,
            TokenKind::Pipe => Self::BitwiseOr,
            TokenKind::Caret => Self::BitwiseXor,
            TokenKind::Ampersand => Self::BitwiseAnd,
            TokenKind::EqEq | TokenKind::BangEq => Self::Equality,
            TokenKind::Lt | TokenKind::LtEq | TokenKind::Gt | TokenKind::GtEq => Self::Comparison,
            TokenKind::Shl | TokenKind::Shr => Self::Shift,
            TokenKind::DotDot | TokenKind::DotDotEq => Self::Range,
            TokenKind::Plus | TokenKind::Minus => Self::Term,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Self::Factor,
            TokenKind::StarStar => Self::Power,
            TokenKind::As => Self::Cast,
            TokenKind::OpenParen | TokenKind::OpenBracket | TokenKind::Dot => Self::Call,
            _ => Self::Lowest,
        }
    }
}

/// Aurora Parser state holding tokens, position, and syntax error reports.
#[derive(Debug)]
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    errors: Vec<ParseError>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
            errors: Vec::new(),
        }
    }

    /// Returns all collected syntax errors.
    #[must_use]
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    /// Consumes the parser and returns collected syntax errors.
    #[must_use]
    pub fn into_errors(self) -> Vec<ParseError> {
        self.errors
    }

    /// Parses an entire source module / file into a `Program` AST.
    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut program = Program::default();

        while !self.is_at_end() {
            // Check for top-level items
            if self.is_item_start() {
                match self.parse_item() {
                    Ok(item) => program.items.push(item),
                    Err(err) => {
                        self.errors.push(err);
                        self.synchronize();
                    }
                }
            } else {
                match self.parse_statement() {
                    Ok(stmt) => program.statements.push(stmt),
                    Err(err) => {
                        self.errors.push(err);
                        self.synchronize();
                    }
                }
            }
        }

        if let Some(err) = self.errors.first().cloned() {
            if program.items.is_empty() && program.statements.is_empty() {
                return Err(err);
            }
        }

        Ok(program)
    }

    fn is_item_start(&self) -> bool {
        match self.peek().kind {
            TokenKind::Fn
            | TokenKind::Struct
            | TokenKind::Enum
            | TokenKind::Trait
            | TokenKind::Impl
            | TokenKind::Type
            | TokenKind::Use => true,
            TokenKind::Pub => matches!(
                self.peek_ahead(1).kind,
                TokenKind::Fn
                    | TokenKind::Struct
                    | TokenKind::Enum
                    | TokenKind::Trait
                    | TokenKind::Type
            ),
            TokenKind::Unsafe | TokenKind::Async => {
                matches!(self.peek_ahead(1).kind, TokenKind::Fn)
            }
            _ => false,
        }
    }

    /// Parses an item declaration (fn, struct, enum, trait, impl, type, use).
    pub fn parse_item(&mut self) -> Result<Item, ParseError> {
        let is_pub = self.match_token(&TokenKind::Pub);
        let is_unsafe = self.match_token(&TokenKind::Unsafe);
        let is_async = self.match_token(&TokenKind::Async);

        match self.peek().kind {
            TokenKind::Fn => {
                let func = self.parse_function_decl(is_pub, is_unsafe, is_async)?;
                let span = func.body.span;
                Ok(Item::new(ItemKind::Function(func), span))
            }
            TokenKind::Struct => {
                let st = self.parse_struct_decl(is_pub)?;
                let span = st.name.span;
                Ok(Item::new(ItemKind::Struct(st), span))
            }
            TokenKind::Enum => {
                let en = self.parse_enum_decl(is_pub)?;
                let span = en.name.span;
                Ok(Item::new(ItemKind::Enum(en), span))
            }
            TokenKind::Trait => {
                let tr = self.parse_trait_decl(is_pub)?;
                let span = tr.name.span;
                Ok(Item::new(ItemKind::Trait(tr), span))
            }
            TokenKind::Impl => {
                let im = self.parse_impl_decl()?;
                let span = im.target_type.span;
                Ok(Item::new(ItemKind::Impl(im), span))
            }
            TokenKind::Type => {
                let ta = self.parse_type_alias_decl(is_pub)?;
                let span = ta.name.span;
                Ok(Item::new(ItemKind::TypeAlias(ta), span))
            }
            TokenKind::Use => {
                let ud = self.parse_use_decl()?;
                let span = ud.path.first().map_or(Span::default(), |id| id.span);
                Ok(Item::new(ItemKind::Use(ud), span))
            }
            _ => {
                let peek = self.peek();
                Err(ParseError::UnexpectedToken {
                    expected: "item declaration (fn, struct, enum, trait, impl, type, use)".into(),
                    found: peek.kind.to_string(),
                    span: peek.span,
                })
            }
        }
    }

    fn parse_function_decl(
        &mut self,
        is_pub: bool,
        is_unsafe: bool,
        is_async: bool,
    ) -> Result<FunctionDecl, ParseError> {
        self.consume(&TokenKind::Fn, "expected 'fn'")?;
        let name = self.parse_ident()?;

        self.consume(&TokenKind::OpenParen, "expected '(' after function name")?;
        let mut params = Vec::new();
        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
            let param_name = self.parse_ident()?;
            self.consume(&TokenKind::Colon, "expected ':' after parameter name")?;
            let param_type = self.parse_type_expr()?;
            let span = param_name.span.merge(param_type.span);
            params.push(Param {
                name: param_name,
                type_annotation: param_type,
                span,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        self.consume(&TokenKind::CloseParen, "expected ')' after parameters")?;

        let return_type = if self.match_token(&TokenKind::Arrow) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(FunctionDecl {
            name,
            params,
            return_type,
            body,
            is_unsafe,
            is_pub,
            is_async,
        })
    }

    fn parse_struct_decl(&mut self, is_pub: bool) -> Result<StructDecl, ParseError> {
        self.consume(&TokenKind::Struct, "expected 'struct'")?;
        let name = self.parse_ident()?;

        self.consume(&TokenKind::OpenBrace, "expected '{' after struct name")?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            let field_pub = self.match_token(&TokenKind::Pub);
            let field_name = self.parse_ident()?;
            self.consume(&TokenKind::Colon, "expected ':' after field name")?;
            let field_type = self.parse_type_expr()?;
            let span = field_name.span.merge(field_type.span);

            fields.push(StructField {
                name: field_name,
                ty: field_type,
                is_pub: field_pub,
                span,
            });

            self.match_token(&TokenKind::Comma);
        }
        self.consume(&TokenKind::CloseBrace, "expected '}' closing struct body")?;

        Ok(StructDecl {
            name,
            fields,
            is_pub,
        })
    }

    fn parse_enum_decl(&mut self, is_pub: bool) -> Result<EnumDecl, ParseError> {
        self.consume(&TokenKind::Enum, "expected 'enum'")?;
        let name = self.parse_ident()?;

        self.consume(&TokenKind::OpenBrace, "expected '{' after enum name")?;
        let mut variants = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            let var_name = self.parse_ident()?;
            let (kind, span) = if self.match_token(&TokenKind::OpenParen) {
                let mut tuple_types = Vec::new();
                while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                    tuple_types.push(self.parse_type_expr()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                let close =
                    self.consume(&TokenKind::CloseParen, "expected ')' after tuple variant")?;
                (
                    EnumVariantKind::Tuple(tuple_types),
                    var_name.span.merge(close.span),
                )
            } else if self.match_token(&TokenKind::OpenBrace) {
                let mut struct_fields = Vec::new();
                while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
                    let fn_name = self.parse_ident()?;
                    self.consume(&TokenKind::Colon, "expected ':'")?;
                    let fty = self.parse_type_expr()?;
                    let fspan = fn_name.span.merge(fty.span);
                    struct_fields.push(StructField {
                        name: fn_name,
                        ty: fty,
                        is_pub: true,
                        span: fspan,
                    });
                    self.match_token(&TokenKind::Comma);
                }
                let close =
                    self.consume(&TokenKind::CloseBrace, "expected '}' after struct variant")?;
                (
                    EnumVariantKind::Struct(struct_fields),
                    var_name.span.merge(close.span),
                )
            } else {
                (EnumVariantKind::Unit, var_name.span)
            };

            variants.push(EnumVariant {
                name: var_name,
                kind,
                span,
            });

            self.match_token(&TokenKind::Comma);
        }
        self.consume(&TokenKind::CloseBrace, "expected '}' closing enum body")?;

        Ok(EnumDecl {
            name,
            variants,
            is_pub,
        })
    }

    fn parse_trait_decl(&mut self, is_pub: bool) -> Result<TraitDecl, ParseError> {
        self.consume(&TokenKind::Trait, "expected 'trait'")?;
        let name = self.parse_ident()?;

        self.consume(&TokenKind::OpenBrace, "expected '{' after trait name")?;
        let mut methods = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            let m_pub = self.match_token(&TokenKind::Pub);
            let m_unsafe = self.match_token(&TokenKind::Unsafe);
            let m_async = self.match_token(&TokenKind::Async);
            let method = self.parse_function_decl(m_pub, m_unsafe, m_async)?;
            methods.push(method);
        }
        self.consume(&TokenKind::CloseBrace, "expected '}' closing trait body")?;

        Ok(TraitDecl {
            name,
            methods,
            is_pub,
        })
    }

    fn parse_impl_decl(&mut self) -> Result<ImplDecl, ParseError> {
        self.consume(&TokenKind::Impl, "expected 'impl'")?;
        let first_type = self.parse_type_expr()?;

        let (trait_name, target_type) = if self.match_token(&TokenKind::For) {
            let TypeExprKind::Named(trait_ident) = first_type.kind else {
                return Err(ParseError::Custom {
                    message: "Trait name in impl block must be an identifier".into(),
                    span: first_type.span,
                });
            };
            let actual_target = self.parse_type_expr()?;
            (Some(trait_ident), actual_target)
        } else {
            (None, first_type)
        };

        self.consume(&TokenKind::OpenBrace, "expected '{' after impl header")?;
        let mut methods = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            let m_pub = self.match_token(&TokenKind::Pub);
            let m_unsafe = self.match_token(&TokenKind::Unsafe);
            let m_async = self.match_token(&TokenKind::Async);
            methods.push(self.parse_function_decl(m_pub, m_unsafe, m_async)?);
        }
        self.consume(&TokenKind::CloseBrace, "expected '}' closing impl body")?;

        Ok(ImplDecl {
            trait_name,
            target_type,
            methods,
        })
    }

    fn parse_type_alias_decl(&mut self, is_pub: bool) -> Result<TypeAliasDecl, ParseError> {
        self.consume(&TokenKind::Type, "expected 'type'")?;
        let name = self.parse_ident()?;
        self.consume(&TokenKind::Equal, "expected '=' after type alias name")?;
        let target = self.parse_type_expr()?;
        self.match_token(&TokenKind::Semicolon);

        Ok(TypeAliasDecl {
            name,
            target,
            is_pub,
        })
    }

    fn parse_use_decl(&mut self) -> Result<UseDecl, ParseError> {
        self.consume(&TokenKind::Use, "expected 'use'")?;
        let mut path = Vec::new();
        let mut is_glob = false;

        loop {
            if self.match_token(&TokenKind::Star) {
                is_glob = true;
                break;
            }

            path.push(self.parse_ident()?);

            if self.match_token(&TokenKind::DoubleColon) {
                continue;
            }
            break;
        }

        self.match_token(&TokenKind::Semicolon);

        Ok(UseDecl { path, is_glob })
    }

    /// Parses a single statement.
    pub fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        if self.check(&TokenKind::Let) {
            self.parse_let_statement()
        } else if self.check(&TokenKind::Const) {
            self.parse_const_statement()
        } else if self.check(&TokenKind::Return) {
            self.parse_return_statement()
        } else if self.is_item_start() {
            let item = self.parse_item()?;
            let span = item.span;
            Ok(Stmt::new(StmtKind::Item(item), span))
        } else {
            let expr = self.parse_expression(Precedence::Lowest)?;
            self.match_token(&TokenKind::Semicolon);
            let span = expr.span;
            Ok(Stmt::new(StmtKind::Expr(expr), span))
        }
    }

    fn parse_let_statement(&mut self) -> Result<Stmt, ParseError> {
        let let_token = self.consume(&TokenKind::Let, "expected 'let'")?;
        let is_mutable = self.match_token(&TokenKind::Mut);
        let name = self.parse_ident()?;

        let type_annotation = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        let initializer = if self.match_token(&TokenKind::Equal) {
            Some(self.parse_expression(Precedence::Lowest)?)
        } else {
            None
        };

        self.match_token(&TokenKind::Semicolon);

        let end_span = initializer.as_ref().map_or(name.span, |expr| expr.span);
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

    fn parse_const_statement(&mut self) -> Result<Stmt, ParseError> {
        let const_token = self.consume(&TokenKind::Const, "expected 'const'")?;
        let name = self.parse_ident()?;

        let type_annotation = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        self.consume(&TokenKind::Equal, "expected '=' for const initializer")?;
        let initializer = self.parse_expression(Precedence::Lowest)?;
        self.match_token(&TokenKind::Semicolon);

        let span = const_token.span.merge(initializer.span);

        Ok(Stmt::new(
            StmtKind::Const {
                name,
                type_annotation,
                initializer,
            },
            span,
        ))
    }

    fn parse_return_statement(&mut self) -> Result<Stmt, ParseError> {
        let ret = self.consume(&TokenKind::Return, "expected 'return'")?;
        let value = if !self.check(&TokenKind::Semicolon)
            && !self.check(&TokenKind::CloseBrace)
            && !self.is_at_end()
        {
            Some(self.parse_expression(Precedence::Lowest)?)
        } else {
            None
        };

        self.match_token(&TokenKind::Semicolon);
        let span = value.as_ref().map_or(ret.span, |v| ret.span.merge(v.span));
        Ok(Stmt::new(StmtKind::Return(value), span))
    }

    /// Parses a block enclosed in curly braces: `{ stmt1; stmt2; expr }`.
    pub fn parse_block(&mut self) -> Result<Block, ParseError> {
        let open = self.consume(&TokenKind::OpenBrace, "expected '{' to open block")?;
        let mut statements = Vec::new();

        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(err) => {
                    self.errors.push(err);
                    self.synchronize();
                }
            }
        }

        let close = self.consume(&TokenKind::CloseBrace, "expected '}' to close block")?;
        let span = open.span.merge(close.span);
        Ok(Block::new(statements, span))
    }

    /// Parses a Type expression (primitives, pointers, references, optionals, arrays, tuples, generics).
    pub fn parse_type_expr(&mut self) -> Result<TypeExpr, ParseError> {
        let start = self.peek().span;

        let mut ty = if self.match_token(&TokenKind::Star) {
            // Raw pointer: *const T or *mut T
            let is_mut = if self.match_token(&TokenKind::Mut) {
                true
            } else {
                self.match_token(&TokenKind::Const);
                false
            };
            let inner = self.parse_type_expr()?;
            let span = start.merge(inner.span);
            TypeExpr::new(
                TypeExprKind::Pointer {
                    inner: Box::new(inner),
                    is_mutable: is_mut,
                },
                span,
            )
        } else if self.match_token(&TokenKind::Ampersand) {
            // Reference: &T or &mut T
            let is_mut = self.match_token(&TokenKind::Mut);
            let inner = self.parse_type_expr()?;
            let span = start.merge(inner.span);
            TypeExpr::new(
                TypeExprKind::Reference {
                    inner: Box::new(inner),
                    is_mutable: is_mut,
                },
                span,
            )
        } else if self.match_token(&TokenKind::OpenBracket) {
            // Array / Slice: [T] or [T; N]
            let elem = self.parse_type_expr()?;
            let size = if self.match_token(&TokenKind::Semicolon) {
                let tok = self.advance();
                if let TokenKind::Int { value, .. } = tok.kind {
                    Some(value as usize)
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "array size integer".into(),
                        found: tok.kind.to_string(),
                        span: tok.span,
                    });
                }
            } else {
                None
            };
            let close =
                self.consume(&TokenKind::CloseBracket, "expected ']' closing array type")?;
            TypeExpr::new(
                TypeExprKind::Array {
                    element: Box::new(elem),
                    size,
                },
                start.merge(close.span),
            )
        } else if self.match_token(&TokenKind::OpenParen) {
            // Unit or Tuple
            if self.match_token(&TokenKind::CloseParen) {
                TypeExpr::new(TypeExprKind::Unit, start.merge(self.previous().span))
            } else {
                let mut types = Vec::new();
                while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                    types.push(self.parse_type_expr()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                let close = self.consume(&TokenKind::CloseParen, "expected ')'")?;
                TypeExpr::new(TypeExprKind::Tuple(types), start.merge(close.span))
            }
        } else {
            let id = self.parse_ident()?;
            // Check for generics: Vec<T>
            if self.match_token(&TokenKind::Lt) {
                let mut args = Vec::new();
                while !self.check(&TokenKind::Gt) && !self.is_at_end() {
                    args.push(self.parse_type_expr()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                let close = self.consume(&TokenKind::Gt, "expected '>' closing generic args")?;
                TypeExpr::new(
                    TypeExprKind::Generic { name: id, args },
                    start.merge(close.span),
                )
            } else {
                TypeExpr::new(TypeExprKind::Named(id.clone()), id.span)
            }
        };

        // Postfix optional check: T?
        while self.match_token(&TokenKind::Question) {
            let span = ty.span.merge(self.previous().span);
            ty = TypeExpr::new(TypeExprKind::Optional(Box::new(ty)), span);
        }

        Ok(ty)
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

    /// Prefix Pratt parsing: handles literals, variables, prefix unary, groupings, blocks, ifs, matches, loops.
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
            TokenKind::Str(s) => Ok(Expr::new(ExprKind::Literal(Literal::String(s)), token.span)),
            TokenKind::RawStr(s) => Ok(Expr::new(
                ExprKind::Literal(Literal::RawString(s)),
                token.span,
            )),
            TokenKind::Char(c) => Ok(Expr::new(ExprKind::Literal(Literal::Char(c)), token.span)),
            TokenKind::Bool(b) => Ok(Expr::new(ExprKind::Literal(Literal::Bool(b)), token.span)),
            TokenKind::Nil => Ok(Expr::new(ExprKind::Literal(Literal::Nil), token.span)),

            TokenKind::Ident(name) => {
                let id = Ident::new(name, token.span);
                // Check if it's a path: ident::ident
                if self.check(&TokenKind::DoubleColon) {
                    let mut path = vec![id];
                    while self.match_token(&TokenKind::DoubleColon) {
                        path.push(self.parse_ident()?);
                    }
                    let end_span = path.last().unwrap().span;
                    Ok(Expr::new(ExprKind::Path(path), token.span.merge(end_span)))
                } else {
                    Ok(Expr::new(ExprKind::Variable(id), token.span))
                }
            }

            // Grouping or Tuple or Lambda
            TokenKind::OpenParen => {
                if self.match_token(&TokenKind::CloseParen) {
                    Ok(Expr::new(
                        ExprKind::Tuple(Vec::new()),
                        token.span.merge(self.previous().span),
                    ))
                } else {
                    let first = self.parse_expression(Precedence::Lowest)?;
                    if self.match_token(&TokenKind::Comma) {
                        // Tuple literal: (x, y, ...)
                        let mut elems = vec![first];
                        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                            elems.push(self.parse_expression(Precedence::Lowest)?);
                            if !self.match_token(&TokenKind::Comma) {
                                break;
                            }
                        }
                        let close =
                            self.consume(&TokenKind::CloseParen, "expected ')' closing tuple")?;
                        Ok(Expr::new(
                            ExprKind::Tuple(elems),
                            token.span.merge(close.span),
                        ))
                    } else {
                        let close = self.consume(&TokenKind::CloseParen, "expected ')'")?;
                        Ok(Expr::new(first.kind, token.span.merge(close.span)))
                    }
                }
            }

            // Array literal: [1, 2, 3]
            TokenKind::OpenBracket => {
                let mut elements = Vec::new();
                while !self.check(&TokenKind::CloseBracket) && !self.is_at_end() {
                    elements.push(self.parse_expression(Precedence::Lowest)?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                let close = self.consume(&TokenKind::CloseBracket, "expected ']' closing array")?;
                Ok(Expr::new(
                    ExprKind::Array(elements),
                    token.span.merge(close.span),
                ))
            }

            // Block expression: { ... }
            TokenKind::OpenBrace => {
                self.position -= 1; // Unconsume '{'
                let block = self.parse_block()?;
                let span = block.span;
                Ok(Expr::new(ExprKind::Block(block), span))
            }

            // Unsafe block: unsafe { ... }
            TokenKind::Unsafe => {
                let block = self.parse_block()?;
                let span = token.span.merge(block.span);
                Ok(Expr::new(ExprKind::UnsafeBlock(block), span))
            }

            // Sizeof / Alignof expressions
            TokenKind::Sizeof => {
                self.consume(&TokenKind::OpenParen, "expected '(' after 'sizeof'")?;
                let ty = self.parse_type_expr()?;
                let close =
                    self.consume(&TokenKind::CloseParen, "expected ')' closing 'sizeof'")?;
                Ok(Expr::new(
                    ExprKind::Sizeof(ty),
                    token.span.merge(close.span),
                ))
            }
            TokenKind::Alignof => {
                self.consume(&TokenKind::OpenParen, "expected '(' after 'alignof'")?;
                let ty = self.parse_type_expr()?;
                let close =
                    self.consume(&TokenKind::CloseParen, "expected ')' closing 'alignof'")?;
                Ok(Expr::new(
                    ExprKind::Alignof(ty),
                    token.span.merge(close.span),
                ))
            }

            // Control flow expressions
            TokenKind::If => self.parse_if_expr(token.span),
            TokenKind::Match => self.parse_match_expr(token.span),
            TokenKind::While => self.parse_while_expr(token.span),
            TokenKind::For => self.parse_for_expr(token.span),
            TokenKind::Loop => {
                let body = self.parse_block()?;
                let span = token.span.merge(body.span);
                Ok(Expr::new(ExprKind::Loop { body }, span))
            }
            TokenKind::Break => {
                let value = if !self.check(&TokenKind::Semicolon)
                    && !self.check(&TokenKind::CloseBrace)
                    && !self.is_at_end()
                {
                    Some(Box::new(self.parse_expression(Precedence::Lowest)?))
                } else {
                    None
                };
                let end_span = value.as_ref().map_or(token.span, |v| v.span);
                Ok(Expr::new(
                    ExprKind::Break(value),
                    token.span.merge(end_span),
                ))
            }
            TokenKind::Continue => Ok(Expr::new(ExprKind::Continue, token.span)),
            TokenKind::Return => {
                let value = if !self.check(&TokenKind::Semicolon)
                    && !self.check(&TokenKind::CloseBrace)
                    && !self.is_at_end()
                {
                    Some(Box::new(self.parse_expression(Precedence::Lowest)?))
                } else {
                    None
                };
                let end_span = value.as_ref().map_or(token.span, |v| v.span);
                Ok(Expr::new(
                    ExprKind::Return(value),
                    token.span.merge(end_span),
                ))
            }

            // Lambda closure expression: |x, y| expr or |x: i32| -> i32 { ... }
            TokenKind::Pipe => self.parse_lambda_expr(token.span),

            // Prefix Unary Operators: -, !, ~, *, &, &mut
            TokenKind::Minus => {
                let operand = self.parse_expression(Precedence::Unary)?;
                let span = token.span.merge(operand.span);
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::Neg,
                        operand: Box::new(operand),
                    },
                    span,
                ))
            }
            TokenKind::Bang => {
                let operand = self.parse_expression(Precedence::Unary)?;
                let span = token.span.merge(operand.span);
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::Not,
                        operand: Box::new(operand),
                    },
                    span,
                ))
            }
            TokenKind::Tilde => {
                let operand = self.parse_expression(Precedence::Unary)?;
                let span = token.span.merge(operand.span);
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::BitNot,
                        operand: Box::new(operand),
                    },
                    span,
                ))
            }
            TokenKind::Star => {
                // Pointer dereference: *ptr
                let operand = self.parse_expression(Precedence::Unary)?;
                let span = token.span.merge(operand.span);
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::Deref,
                        operand: Box::new(operand),
                    },
                    span,
                ))
            }
            TokenKind::Ampersand => {
                // Address-of: &val or &mut val
                let op = if self.match_token(&TokenKind::Mut) {
                    UnaryOp::AddressOfMut
                } else {
                    UnaryOp::AddressOf
                };
                let operand = self.parse_expression(Precedence::Unary)?;
                let span = token.span.merge(operand.span);
                Ok(Expr::new(
                    ExprKind::Unary {
                        op,
                        operand: Box::new(operand),
                    },
                    span,
                ))
            }

            // Prefix Range: ..10 or ..=10
            TokenKind::DotDot => {
                let end = self.parse_expression(Precedence::Range)?;
                let span = token.span.merge(end.span);
                Ok(Expr::new(
                    ExprKind::Range {
                        start: None,
                        end: Some(Box::new(end)),
                        inclusive: false,
                    },
                    span,
                ))
            }
            TokenKind::DotDotEq => {
                let end = self.parse_expression(Precedence::Range)?;
                let span = token.span.merge(end.span);
                Ok(Expr::new(
                    ExprKind::Range {
                        start: None,
                        end: Some(Box::new(end)),
                        inclusive: true,
                    },
                    span,
                ))
            }

            other => Err(ParseError::UnexpectedToken {
                expected: "expression".into(),
                found: other.to_string(),
                span: token.span,
            }),
        }
    }

    /// Infix Pratt parsing: binary operators, assignments, pipeline, member access, index, calls, casts.
    fn parse_infix(&mut self, left: Expr, prec: Precedence) -> Result<Expr, ParseError> {
        let op_token = self.advance();

        match op_token.kind {
            // Function call: expr(args)
            TokenKind::OpenParen => {
                let mut args = Vec::new();
                while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                    args.push(self.parse_expression(Precedence::Lowest)?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                let close = self.consume(&TokenKind::CloseParen, "expected ')' closing call")?;
                let span = left.span.merge(close.span);
                Ok(Expr::new(
                    ExprKind::Call {
                        callee: Box::new(left),
                        args,
                    },
                    span,
                ))
            }

            // Array index: expr[idx]
            TokenKind::OpenBracket => {
                let index = self.parse_expression(Precedence::Lowest)?;
                let close = self.consume(&TokenKind::CloseBracket, "expected ']' closing index")?;
                let span = left.span.merge(close.span);
                Ok(Expr::new(
                    ExprKind::Index {
                        target: Box::new(left),
                        index: Box::new(index),
                    },
                    span,
                ))
            }

            // Member access: expr.ident
            TokenKind::Dot => {
                let member = self.parse_ident()?;
                let span = left.span.merge(member.span);
                Ok(Expr::new(
                    ExprKind::MemberAccess {
                        object: Box::new(left),
                        member,
                    },
                    span,
                ))
            }

            // Type cast: expr as TargetType
            TokenKind::As => {
                let target_type = self.parse_type_expr()?;
                let span = left.span.merge(target_type.span);
                Ok(Expr::new(
                    ExprKind::Cast {
                        expr: Box::new(left),
                        target_type,
                    },
                    span,
                ))
            }

            // Range infix: start..end or start..=end
            TokenKind::DotDot => {
                let end = if self.is_range_end() {
                    None
                } else {
                    Some(Box::new(self.parse_expression(prec)?))
                };
                let end_span = end.as_ref().map_or(op_token.span, |e| e.span);
                let span = left.span.merge(end_span);
                Ok(Expr::new(
                    ExprKind::Range {
                        start: Some(Box::new(left)),
                        end,
                        inclusive: false,
                    },
                    span,
                ))
            }
            TokenKind::DotDotEq => {
                let end = self.parse_expression(prec)?;
                let span = left.span.merge(end.span);
                Ok(Expr::new(
                    ExprKind::Range {
                        start: Some(Box::new(left)),
                        end: Some(Box::new(end)),
                        inclusive: true,
                    },
                    span,
                ))
            }

            // Assignment and Compound Assignments
            TokenKind::Equal => self.parse_assignment(left, AssignOp::Assign),
            TokenKind::PlusEq => self.parse_assignment(left, AssignOp::AddAssign),
            TokenKind::MinusEq => self.parse_assignment(left, AssignOp::SubAssign),
            TokenKind::StarEq => self.parse_assignment(left, AssignOp::MulAssign),
            TokenKind::SlashEq => self.parse_assignment(left, AssignOp::DivAssign),
            TokenKind::PercentEq => self.parse_assignment(left, AssignOp::ModAssign),
            TokenKind::AmpEq => self.parse_assignment(left, AssignOp::BitAndAssign),
            TokenKind::PipeEq => self.parse_assignment(left, AssignOp::BitOrAssign),
            TokenKind::CaretEq => self.parse_assignment(left, AssignOp::BitXorAssign),
            TokenKind::ShlEq => self.parse_assignment(left, AssignOp::ShlAssign),
            TokenKind::ShrEq => self.parse_assignment(left, AssignOp::ShrAssign),

            // Binary Operators
            _ => {
                let op = match op_token.kind {
                    TokenKind::Plus => BinaryOp::Add,
                    TokenKind::Minus => BinaryOp::Sub,
                    TokenKind::Star => BinaryOp::Mul,
                    TokenKind::Slash => BinaryOp::Div,
                    TokenKind::Percent => BinaryOp::Mod,
                    TokenKind::StarStar => BinaryOp::Power,
                    TokenKind::EqEq => BinaryOp::Eq,
                    TokenKind::BangEq => BinaryOp::NotEq,
                    TokenKind::Lt => BinaryOp::Lt,
                    TokenKind::LtEq => BinaryOp::LtEq,
                    TokenKind::Gt => BinaryOp::Gt,
                    TokenKind::GtEq => BinaryOp::GtEq,
                    TokenKind::AmpAmp => BinaryOp::And,
                    TokenKind::PipePipe => BinaryOp::Or,
                    TokenKind::PipeGt => BinaryOp::Pipe,
                    TokenKind::QuestionQuestion => BinaryOp::NullCoalesce,
                    TokenKind::Ampersand => BinaryOp::BitAnd,
                    TokenKind::Pipe => BinaryOp::BitOr,
                    TokenKind::Caret => BinaryOp::BitXor,
                    TokenKind::Shl => BinaryOp::Shl,
                    TokenKind::Shr => BinaryOp::Shr,
                    _ => {
                        return Err(ParseError::UnexpectedToken {
                            expected: "binary operator".into(),
                            found: op_token.kind.to_string(),
                            span: op_token.span,
                        })
                    }
                };

                // Power operator (**) is right-associative
                let next_prec = if op == BinaryOp::Power {
                    Precedence::Factor
                } else {
                    prec
                };

                let right = self.parse_expression(next_prec)?;
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
        }
    }

    fn parse_assignment(&mut self, target: Expr, op: AssignOp) -> Result<Expr, ParseError> {
        // Verify valid lvalue target: Variable, MemberAccess, Index, or Deref
        match &target.kind {
            ExprKind::Variable(_)
            | ExprKind::MemberAccess { .. }
            | ExprKind::Index { .. }
            | ExprKind::Unary {
                op: UnaryOp::Deref, ..
            } => {}
            _ => return Err(ParseError::InvalidAssignmentTarget { span: target.span }),
        }

        let value = self.parse_expression(Precedence::Assignment)?;
        let span = target.span.merge(value.span);
        Ok(Expr::new(
            ExprKind::Assignment {
                target: Box::new(target),
                op,
                value: Box::new(value),
            },
            span,
        ))
    }

    fn is_range_end(&self) -> bool {
        self.check(&TokenKind::Semicolon)
            || self.check(&TokenKind::CloseBrace)
            || self.check(&TokenKind::CloseParen)
            || self.check(&TokenKind::CloseBracket)
            || self.check(&TokenKind::Comma)
            || self.is_at_end()
    }

    fn parse_if_expr(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        let condition = self.parse_expression(Precedence::Lowest)?;
        let then_branch = self.parse_block()?;

        let else_branch = if self.match_token(&TokenKind::Else) {
            if self.check(&TokenKind::If) {
                let if_tok = self.advance();
                Some(Box::new(self.parse_if_expr(if_tok.span)?))
            } else {
                let else_block = self.parse_block()?;
                let span = else_block.span;
                Some(Box::new(Expr::new(ExprKind::Block(else_block), span)))
            }
        } else {
            None
        };

        let end_span = else_branch.as_ref().map_or(then_branch.span, |b| b.span);
        let span = start_span.merge(end_span);

        Ok(Expr::new(
            ExprKind::If {
                condition: Box::new(condition),
                then_branch,
                else_branch,
            },
            span,
        ))
    }

    fn parse_match_expr(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        let value = self.parse_expression(Precedence::Lowest)?;
        self.consume(&TokenKind::OpenBrace, "expected '{' after match target")?;

        let mut arms = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            let pattern = self.parse_pattern()?;
            let guard = if self.match_token(&TokenKind::If) {
                Some(self.parse_expression(Precedence::Lowest)?)
            } else {
                None
            };

            self.consume(&TokenKind::FatArrow, "expected '=>' after match pattern")?;

            let body = if self.check(&TokenKind::OpenBrace) {
                let block = self.parse_block()?;
                let bspan = block.span;
                Expr::new(ExprKind::Block(block), bspan)
            } else {
                let expr = self.parse_expression(Precedence::Lowest)?;
                self.match_token(&TokenKind::Comma);
                expr
            };

            let arm_span = pattern.span.merge(body.span);
            arms.push(MatchArm {
                pattern,
                guard,
                body,
                span: arm_span,
            });

            self.match_token(&TokenKind::Comma);
        }

        let close = self.consume(&TokenKind::CloseBrace, "expected '}' closing match")?;
        let span = start_span.merge(close.span);

        Ok(Expr::new(
            ExprKind::Match {
                value: Box::new(value),
                arms,
            },
            span,
        ))
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        let tok = self.advance();
        match tok.kind {
            TokenKind::Ident(ref name) if name == "_" => {
                Ok(Pattern::new(PatternKind::Wildcard, tok.span))
            }
            TokenKind::Ident(name) => {
                let id = Ident::new(name, tok.span);
                if self.check(&TokenKind::DoubleColon) {
                    let mut path = vec![id];
                    while self.match_token(&TokenKind::DoubleColon) {
                        path.push(self.parse_ident()?);
                    }
                    let end_span = path.last().unwrap().span;
                    Ok(Pattern::new(
                        PatternKind::Path(path),
                        tok.span.merge(end_span),
                    ))
                } else {
                    Ok(Pattern::new(PatternKind::Variable(id), tok.span))
                }
            }
            TokenKind::Int { value, .. } => Ok(Pattern::new(
                PatternKind::Literal(Literal::Int(value)),
                tok.span,
            )),
            TokenKind::Str(s) => Ok(Pattern::new(
                PatternKind::Literal(Literal::String(s)),
                tok.span,
            )),
            TokenKind::Bool(b) => Ok(Pattern::new(
                PatternKind::Literal(Literal::Bool(b)),
                tok.span,
            )),
            TokenKind::Nil => Ok(Pattern::new(PatternKind::Literal(Literal::Nil), tok.span)),
            TokenKind::OpenParen => {
                let mut pats = Vec::new();
                while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                    pats.push(self.parse_pattern()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                let close =
                    self.consume(&TokenKind::CloseParen, "expected ')' closing pattern tuple")?;
                Ok(Pattern::new(
                    PatternKind::Tuple(pats),
                    tok.span.merge(close.span),
                ))
            }
            other => Err(ParseError::UnexpectedToken {
                expected: "pattern (literal, identifier, wildcard '_', tuple)".into(),
                found: other.to_string(),
                span: tok.span,
            }),
        }
    }

    fn parse_while_expr(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        let condition = self.parse_expression(Precedence::Lowest)?;
        let body = self.parse_block()?;
        let span = start_span.merge(body.span);
        Ok(Expr::new(
            ExprKind::While {
                condition: Box::new(condition),
                body,
            },
            span,
        ))
    }

    fn parse_for_expr(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        let var = self.parse_ident()?;
        self.consume(&TokenKind::In, "expected 'in' in for loop")?;
        let iter = self.parse_expression(Precedence::Lowest)?;
        let body = self.parse_block()?;
        let span = start_span.merge(body.span);
        Ok(Expr::new(
            ExprKind::For {
                var,
                iter: Box::new(iter),
                body,
            },
            span,
        ))
    }

    fn parse_lambda_expr(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        let mut params = Vec::new();

        while !self.check(&TokenKind::Pipe) && !self.is_at_end() {
            let name = self.parse_ident()?;
            let type_annotation = if self.match_token(&TokenKind::Colon) {
                self.parse_type_expr()?
            } else {
                TypeExpr::new(
                    TypeExprKind::Named(Ident::new("auto", name.span)),
                    name.span,
                )
            };
            let pspan = name.span.merge(type_annotation.span);
            params.push(Param {
                name,
                type_annotation,
                span: pspan,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        self.consume(
            &TokenKind::Pipe,
            "expected closing '|' in lambda parameter list",
        )?;

        let return_type = if self.match_token(&TokenKind::Arrow) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        let body = if self.check(&TokenKind::OpenBrace) {
            let block = self.parse_block()?;
            let bspan = block.span;
            Box::new(Expr::new(ExprKind::Block(block), bspan))
        } else {
            Box::new(self.parse_expression(Precedence::Lowest)?)
        };

        let span = start_span.merge(body.span);
        Ok(Expr::new(
            ExprKind::Lambda {
                params,
                return_type,
                body,
            },
            span,
        ))
    }

    pub fn parse_ident(&mut self) -> Result<Ident, ParseError> {
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

    /// Error recovery synchronization: advances past the offending token sequence until a safe statement/item boundary.
    pub fn synchronize(&mut self) {
        while !self.is_at_end() {
            if self.previous().kind == TokenKind::Semicolon {
                return;
            }

            match self.peek().kind {
                TokenKind::Fn
                | TokenKind::Let
                | TokenKind::Const
                | TokenKind::Struct
                | TokenKind::Enum
                | TokenKind::Trait
                | TokenKind::Impl
                | TokenKind::Type
                | TokenKind::Use
                | TokenKind::If
                | TokenKind::While
                | TokenKind::For
                | TokenKind::Loop
                | TokenKind::Return
                | TokenKind::Unsafe => return,
                _ => {
                    self.advance();
                }
            }
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

    fn peek_ahead(&self, offset: usize) -> &Token {
        let idx = (self.position + offset).min(self.tokens.len().saturating_sub(1));
        &self.tokens[idx]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.position.saturating_sub(1)]
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
    fn test_parse_arithmetic_precedence() {
        let code = "let total = 10 + 20 * 3 ** 2";
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
                // Precedence: 3 ** 2 binds tightest, then 20 * (3 ** 2), then 10 + (...)
                if let ExprKind::Binary { op, left, right } = expr.kind {
                    assert_eq!(op, BinaryOp::Add);
                    assert_eq!(left.kind, ExprKind::Literal(Literal::Int(10)));
                    if let ExprKind::Binary { op: op2, .. } = right.kind {
                        assert_eq!(op2, BinaryOp::Mul);
                    } else {
                        panic!("Expected multiplication right branch");
                    }
                } else {
                    panic!("Expected binary addition");
                }
            }
            _ => panic!("Expected let statement"),
        }
    }

    #[test]
    fn test_parse_function_and_pipeline() {
        let code = r"
            fn transform(data: [i32]) -> i32 {
                return data |> filter() |> sum()
            }
        ";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();

        assert_eq!(program.items.len(), 1);
        match &program.items[0].kind {
            ItemKind::Function(f) => {
                assert_eq!(f.name.name, "transform");
                assert_eq!(f.params.len(), 1);
                assert_eq!(f.body.statements.len(), 1);
            }
            _ => panic!("Expected function item"),
        }
    }

    #[test]
    fn test_parse_struct_and_enum() {
        let code = r"
            struct Point {
                pub x: f64,
                pub y: f64,
            }

            enum Result {
                Ok(i32),
                Err(string),
            }
        ";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();

        assert_eq!(program.items.len(), 2);
    }

    #[test]
    fn test_parse_unsafe_and_pointer() {
        let code = r"
            unsafe fn memcopy(dest: *mut u8, src: *const u8, len: usize) {
                *dest = *src;
            }
        ";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();

        assert_eq!(program.items.len(), 1);
        if let ItemKind::Function(f) = &program.items[0].kind {
            assert!(f.is_unsafe);
            assert_eq!(f.params.len(), 3);
        } else {
            panic!("Expected unsafe function");
        }
    }

    #[test]
    fn test_parse_match_expression() {
        let code = r#"
            let res = match status {
                0 => "ok",
                1 => "warn",
                _ => "error"
            }
        "#;
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_statement().unwrap();

        if let StmtKind::Let {
            initializer: Some(expr),
            ..
        } = stmt.kind
        {
            if let ExprKind::Match { arms, .. } = expr.kind {
                assert_eq!(arms.len(), 3);
            } else {
                panic!("Expected match expression");
            }
        }
    }
}
