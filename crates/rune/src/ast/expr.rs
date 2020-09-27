use crate::ast;
use crate::{
    OptionSpanned as _, Parse, ParseError, ParseErrorKind, Parser, Peek, Spanned, ToTokens,
};
use runestick::Span;
use std::mem::take;
use std::ops;

/// Indicator that an expression should be parsed with an eager brace.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EagerBrace(pub(crate) bool);

impl ops::Deref for EagerBrace {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A rune expression.
#[derive(Debug, Clone, ToTokens, Spanned, PartialEq, Eq)]
pub enum Expr {
    /// The `self` keyword.
    Self_(ast::Self_),
    /// An path expression.
    Path(ast::Path),
    /// A declaration.
    Item(ast::Item),
    /// A while loop.
    ExprWhile(ast::ExprWhile),
    /// An unconditional loop.
    ExprLoop(ast::ExprLoop),
    /// An for loop.
    ExprFor(ast::ExprFor),
    /// A let expression.
    ExprLet(ast::ExprLet),
    /// An index set operation.
    ExprIndexSet(ast::ExprIndexSet),
    /// An if expression.
    ExprIf(ast::ExprIf),
    /// An match expression.
    ExprMatch(ast::ExprMatch),
    /// A function call,
    ExprCall(ast::ExprCall),
    /// A macro call,
    MacroCall(ast::MacroCall),
    /// A field access on an expression.
    ExprFieldAccess(ast::ExprFieldAccess),
    /// A grouped expression.
    ExprGroup(ast::ExprGroup),
    /// A binary expression.
    ExprBinary(ast::ExprBinary),
    /// A unary expression.
    ExprUnary(ast::ExprUnary),
    /// An index set operation.
    ExprIndexGet(ast::ExprIndexGet),
    /// A break expression.
    ExprBreak(ast::ExprBreak),
    /// A yield expression.
    ExprYield(ast::ExprYield),
    /// A block as an expression.
    ExprBlock(ast::ExprBlock),
    /// A return statement.
    ExprReturn(ast::ExprReturn),
    /// An await expression.
    ExprAwait(ast::ExprAwait),
    /// Try expression.
    ExprTry(ast::ExprTry),
    /// A select expression.
    ExprSelect(ast::ExprSelect),
    /// A closure expression.
    ExprClosure(ast::ExprClosure),
    /// A literal expression.
    ExprLit(ast::ExprLit),
}

impl Expr {
    /// Indicates if an expression needs a semicolon or must be last in a block.
    pub fn needs_semi(&self) -> bool {
        match self {
            Self::ExprWhile(_) => false,
            Self::ExprLoop(_) => false,
            Self::ExprFor(_) => false,
            Self::ExprIf(_) => false,
            Self::ExprMatch(_) => false,
            Self::ExprBlock(_) => false,
            Self::ExprSelect(_) => false,
            _ => true,
        }
    }

    /// Test if expression should be chained by default.
    pub fn is_chainable(&self) -> bool {
        match self {
            Self::ExprWhile(..) => false,
            Self::ExprLoop(..) => false,
            Self::ExprFor(..) => false,
            _ => true,
        }
    }

    /// Test if the expression has any attributes
    pub fn attributes_span(&self) -> Option<Span> {
        match self {
            Expr::Item(expr) => expr.attributes_span(),
            Expr::ExprBreak(expr) => expr.attributes.option_span(),
            Expr::ExprYield(expr) => expr.attributes.option_span(),
            Expr::ExprBlock(expr) => expr.attributes.option_span(),
            Expr::ExprReturn(expr) => expr.attributes.option_span(),
            Expr::ExprClosure(expr) => expr.attributes.option_span(),
            Expr::ExprMatch(expr) => expr.attributes.option_span(),
            Expr::ExprWhile(expr) => expr.attributes.option_span(),
            Expr::ExprLoop(expr) => expr.attributes.option_span(),
            Expr::ExprFor(expr) => expr.attributes.option_span(),
            Expr::ExprLet(expr) => expr.attributes.option_span(),
            Expr::ExprIf(expr) => expr.attributes.option_span(),
            Expr::ExprSelect(expr) => expr.attributes.option_span(),
            Expr::ExprLit(expr) => expr.attributes.option_span(),
            Expr::ExprAwait(_)
            | Expr::ExprCall(_)
            | Expr::Self_(_)
            | Expr::Path(_)
            | Expr::ExprIndexSet(_)
            | Expr::MacroCall(_)
            | Expr::ExprFieldAccess(_)
            | Expr::ExprGroup(_)
            | Expr::ExprBinary(_)
            | Expr::ExprUnary(_)
            | Expr::ExprIndexGet(_)
            | Expr::ExprTry(_) => None,
        }
    }

    /// Parse an expression without an eager brace.
    ///
    /// This is used to solve a syntax ambiguity when parsing expressions that
    /// are arguments to statements immediately followed by blocks. Like `if`,
    /// `while`, and `match`.
    pub(crate) fn parse_without_eager_brace(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Self::parse_with(parser, EagerBrace(false))
    }

    /// ull, configurable parsing of an expression.F
    fn parse_with(parser: &mut Parser<'_>, eager_brace: EagerBrace) -> Result<Self, ParseError> {
        let mut attributes = parser.parse()?;

        let expr = Self::parse_base(parser, &mut attributes, eager_brace)?;
        let expr = Self::parse_chain(parser, expr)?;
        let expr = Self::parse_binary(parser, expr, 0, eager_brace)?;

        if let Some(span) = attributes.option_span() {
            return Err(ParseError::new(
                span,
                ParseErrorKind::AttributesNotSupported,
            ));
        }

        Ok(expr)
    }

    /// Parse expressions that start with an identifier.
    pub(crate) fn parse_with_meta_path(
        parser: &mut Parser<'_>,
        attributes: &mut Vec<ast::Attribute>,
        path: ast::Path,
        eager_brace: EagerBrace,
    ) -> Result<Self, ParseError> {
        if *eager_brace && parser.peek::<ast::OpenBrace>()? {
            let ident = ast::LitObjectIdent::Named(path);

            return Ok(Self::ExprLit(ast::ExprLit {
                attributes: take(attributes),
                lit: ast::Lit::Object(ast::LitObject::parse_with_ident(parser, ident)?),
            }));
        }

        if parser.peek::<ast::Bang>()? {
            return Ok(Self::MacroCall(ast::MacroCall::parse_with_meta_path(
                parser,
                std::mem::take(attributes),
                path,
            )?));
        }

        Ok(Self::Path(path))
    }

    /// Parsing something that opens with a parenthesis.
    pub fn parse_open_paren(
        parser: &mut Parser<'_>,
        attributes: &mut Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        if parser.peek::<ast::LitUnit>()? {
            return Ok(Self::ExprLit(ast::ExprLit {
                attributes: take(attributes),
                lit: parser.parse()?,
            }));
        }

        let open = parser.parse::<ast::OpenParen>()?;
        let expr = ast::Expr::parse_with(parser, EagerBrace(true))?;

        if parser.peek::<ast::CloseParen>()? {
            return Ok(Expr::ExprGroup(ast::ExprGroup {
                open,
                expr: Box::new(expr),
                close: parser.parse()?,
            }));
        }

        let tuple = ast::LitTuple::parse_from_first_expr(parser, open, expr)?;

        Ok(Expr::ExprLit(ast::ExprLit {
            attributes: take(attributes),
            lit: ast::Lit::Tuple(tuple),
        }))
    }

    pub(crate) fn parse_with_meta(
        parser: &mut Parser<'_>,
        attributes: &mut Vec<ast::Attribute>,
        path: Option<ast::Path>,
    ) -> Result<Self, ParseError> {
        let lhs = if let Some(path) = path {
            Self::parse_with_meta_path(parser, attributes, path, EagerBrace(true))?
        } else {
            Self::parse_base(parser, attributes, EagerBrace(true))?
        };

        let lhs = Self::parse_chain(parser, lhs)?;
        Ok(Self::parse_binary(parser, lhs, 0, EagerBrace(true))?)
    }

    /// Parse a basic expression.
    fn parse_base(
        parser: &mut Parser<'_>,
        attributes: &mut Vec<ast::Attribute>,
        eager_brace: EagerBrace,
    ) -> Result<Self, ParseError> {
        if let Some(path) = parser.parse::<Option<ast::Path>>()? {
            return Ok(Self::parse_with_meta_path(
                parser,
                attributes,
                path,
                eager_brace,
            )?);
        }

        if ast::Lit::peek_in_expr(parser)? {
            return Ok(ast::Expr::ExprLit(ast::ExprLit::parse_with_attributes(
                parser,
                take(attributes),
            )?));
        }

        let mut label = parser.parse::<Option<(ast::Label, ast::Colon)>>()?;
        let mut async_token = parser.parse::<Option<ast::Async>>()?;
        let t = parser.token_peek_eof()?;

        let expr = match t.kind {
            ast::Kind::PipePipe | ast::Kind::Pipe => {
                Self::ExprClosure(ast::ExprClosure::parse_with_attributes_and_async(
                    parser,
                    take(attributes),
                    take(&mut async_token),
                )?)
            }
            ast::Kind::Self_ => Self::Self_(parser.parse()?),
            ast::Kind::Select => Self::ExprSelect(ast::ExprSelect::parse_with_attributes(
                parser,
                take(attributes),
            )?),
            ast::Kind::Bang | ast::Kind::Amp | ast::Kind::Star => Self::ExprUnary(parser.parse()?),
            ast::Kind::While => Self::ExprWhile(ast::ExprWhile::parse_with_attributes_and_label(
                parser,
                take(attributes),
                take(&mut label),
            )?),
            ast::Kind::Loop => Self::ExprLoop(ast::ExprLoop::parse_with_attributes_and_label(
                parser,
                take(attributes),
                take(&mut label),
            )?),
            ast::Kind::For => Self::ExprFor(ast::ExprFor::parse_with_attributes_and_label(
                parser,
                take(attributes),
                take(&mut label),
            )?),
            ast::Kind::Let => Self::ExprLet(ast::ExprLet::parse_with_attributes(
                parser,
                take(attributes),
            )?),
            ast::Kind::If => Self::ExprIf(ast::ExprIf::parse_with_attributes(
                parser,
                take(attributes),
            )?),
            ast::Kind::Match => Self::ExprMatch(ast::ExprMatch::parse_with_attributes(
                parser,
                take(attributes),
            )?),
            ast::Kind::Open(ast::Delimiter::Parenthesis) => {
                Self::parse_open_paren(parser, attributes)?
            }
            ast::Kind::Open(ast::Delimiter::Brace) => Self::ExprBlock(ast::ExprBlock {
                async_token: take(&mut async_token),
                attributes: take(attributes),
                block: parser.parse()?,
            }),
            ast::Kind::Break => Self::ExprBreak(ast::ExprBreak::parse_with_attributes(
                parser,
                take(attributes),
            )?),
            ast::Kind::Yield => Self::ExprYield(ast::ExprYield::parse_with_attributes(
                parser,
                take(attributes),
            )?),
            ast::Kind::Return => Self::ExprReturn(ast::ExprReturn::parse_with_attributes(
                parser,
                take(attributes),
            )?),
            _ => {
                return Err(ParseError::expected(t, "expression"));
            }
        };

        if let Some(span) = label.option_span() {
            return Err(ParseError::new(span, ParseErrorKind::UnsupportedLabel));
        }

        if let Some(span) = async_token.option_span() {
            return Err(ParseError::new(span, ParseErrorKind::UnsupportedAsync));
        }

        Ok(expr)
    }

    /// Parse an expression chain.
    fn parse_chain(parser: &mut Parser<'_>, mut expr: Self) -> Result<Self, ParseError> {
        while let Some(token) = parser.token_peek()? {
            let is_chainable = expr.is_chainable();

            match token.kind {
                ast::Kind::Open(ast::Delimiter::Bracket) if is_chainable => {
                    let index_get = ast::ExprIndexGet {
                        target: Box::new(expr),
                        open: parser.parse()?,
                        index: parser.parse()?,
                        close: parser.parse()?,
                    };

                    if parser.peek::<ast::Eq>()? {
                        return Ok(Self::ExprIndexSet(ast::ExprIndexSet {
                            target: index_get.target,
                            open: index_get.open,
                            index: index_get.index,
                            close: index_get.close,
                            eq: parser.parse()?,
                            value: parser.parse()?,
                        }));
                    }

                    expr = Self::ExprIndexGet(index_get);
                }
                // Chained function call.
                ast::Kind::Open(ast::Delimiter::Parenthesis) if is_chainable => {
                    let args = parser.parse::<ast::Parenthesized<ast::Expr, ast::Comma>>()?;

                    expr = Expr::ExprCall(ast::ExprCall {
                        expr: Box::new(expr),
                        args,
                    });
                }
                ast::Kind::QuestionMark => {
                    expr = Expr::ExprTry(ast::ExprTry {
                        expr: Box::new(expr),
                        try_: parser.parse()?,
                    });
                }
                ast::Kind::Dot => {
                    let dot = parser.parse()?;

                    if let Some(t) = parser.token_peek()? {
                        if let ast::Kind::Await = t.kind {
                            expr = Expr::ExprAwait(ast::ExprAwait {
                                expr: Box::new(expr),
                                dot,
                                await_: parser.parse()?,
                            });

                            continue;
                        }
                    }

                    let next = Expr::parse_base(parser, &mut vec![], EagerBrace(false))?;

                    let span = match next {
                        Expr::Path(path) => {
                            let span = path.span();

                            if let Some(name) = path.try_as_ident() {
                                expr = Expr::ExprFieldAccess(ast::ExprFieldAccess {
                                    expr: Box::new(expr),
                                    dot,
                                    expr_field: ast::ExprField::Ident(name.clone()),
                                });

                                continue;
                            }

                            span
                        }
                        Expr::ExprLit(ast::ExprLit {
                            lit: ast::Lit::Number(n),
                            attributes,
                        }) if attributes.is_empty() => {
                            expr = Expr::ExprFieldAccess(ast::ExprFieldAccess {
                                expr: Box::new(expr),
                                dot,
                                expr_field: ast::ExprField::LitNumber(n),
                            });

                            continue;
                        }
                        other => other.span(),
                    };

                    return Err(ParseError::new(
                        span,
                        ParseErrorKind::UnsupportedFieldAccess,
                    ));
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse a binary expression.
    fn parse_binary(
        parser: &mut Parser<'_>,
        mut lhs: Self,
        min_precedence: usize,
        eager_brace: EagerBrace,
    ) -> Result<Self, ParseError> {
        let mut lookahead_tok = parser.token_peek_pair()?;

        loop {
            let lookahead = lookahead_tok.and_then(ast::BinOp::from_token);

            let (op, t1, t2) = match lookahead {
                Some((op, t1, t2)) if op.precedence() >= min_precedence => (op, t1, t2),
                _ => break,
            };

            for _ in 0..op.advance() {
                parser.token_next()?;
            }

            let rhs = Self::parse_base(parser, &mut vec![], eager_brace)?;
            let mut rhs = Self::parse_chain(parser, rhs)?;

            lookahead_tok = parser.token_peek_pair()?;

            loop {
                let lh = match lookahead_tok.and_then(ast::BinOp::from_token) {
                    Some((lh, _, _)) if lh.precedence() > op.precedence() => lh,
                    Some((lh, _, _)) if lh.precedence() == op.precedence() && !op.is_assoc() => {
                        return Err(ParseError::new(
                            lhs.span().join(rhs.span()),
                            ParseErrorKind::PrecedenceGroupRequired,
                        ));
                    }
                    _ => break,
                };

                rhs = Self::parse_binary(parser, rhs, lh.precedence(), eager_brace)?;
                lookahead_tok = parser.token_peek_pair()?;
            }

            lhs = Expr::ExprBinary(ast::ExprBinary {
                lhs: Box::new(lhs),
                t1,
                t2,
                rhs: Box::new(rhs),
                op,
            });
        }

        Ok(lhs)
    }
}

/// Parsing a block expression.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::Expr>("foo[\"foo\"]");
/// testing::roundtrip::<ast::Expr>("foo.bar()");
/// testing::roundtrip::<ast::Expr>("var()");
/// testing::roundtrip::<ast::Expr>("var");
/// testing::roundtrip::<ast::Expr>("42");
/// testing::roundtrip::<ast::Expr>("1 + 2 / 3 - 4 * 1");
/// testing::roundtrip::<ast::Expr>("foo[\"bar\"]");
/// testing::roundtrip::<ast::Expr>("let var = 42");
/// testing::roundtrip::<ast::Expr>("let var = \"foo bar\"");
/// testing::roundtrip::<ast::Expr>("var[\"foo\"] = \"bar\"");
/// testing::roundtrip::<ast::Expr>("let var = objects[\"foo\"] + 1");
/// testing::roundtrip::<ast::Expr>("var = 42");
///
/// let expr = testing::roundtrip::<ast::Expr>(r#"
///     if 1 { } else { if 2 { } else { } }
/// "#);
/// assert!(matches!(expr, ast::Expr::ExprIf(..)));
///
/// // Chained function calls.
/// testing::roundtrip::<ast::Expr>("foo.bar.baz()");
/// testing::roundtrip::<ast::Expr>("foo[0][1][2]");
/// testing::roundtrip::<ast::Expr>("foo.bar()[0].baz()[1]");
///
/// testing::roundtrip::<ast::Expr>("42 is int::int");
/// testing::roundtrip::<ast::Expr>("{ let x = 1; x }");
///
/// let expr = testing::roundtrip::<ast::Expr>("#[cfg(debug_assertions)] { assert_eq(x, 32); }");
/// assert!(matches!(expr, ast::Expr::ExprBlock(b) if b.attributes.len() == 1 && b.block.statements.len() == 1));
/// ```
impl Parse for Expr {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Self::parse_with(parser, EagerBrace(true))
    }
}

impl Peek for Expr {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        match peek!(t1).kind {
            ast::Kind::Async => true,
            ast::Kind::Self_ => true,
            ast::Kind::Select => true,
            ast::Kind::Label(..) => matches!(peek!(t2).kind, ast::Kind::Colon),
            ast::Kind::Pound => true,
            ast::Kind::Bang | ast::Kind::Amp | ast::Kind::Star => true,
            ast::Kind::While => true,
            ast::Kind::Loop => true,
            ast::Kind::For => true,
            ast::Kind::Let => true,
            ast::Kind::If => true,
            ast::Kind::LitNumber { .. } => true,
            ast::Kind::LitChar { .. } => true,
            ast::Kind::LitByte { .. } => true,
            ast::Kind::LitStr { .. } => true,
            ast::Kind::LitByteStr { .. } => true,
            ast::Kind::LitTemplate { .. } => true,
            ast::Kind::Open(ast::Delimiter::Parenthesis) => true,
            ast::Kind::Open(ast::Delimiter::Bracket) => true,
            ast::Kind::Open(ast::Delimiter::Brace) => true,
            ast::Kind::True | ast::Kind::False => true,
            ast::Kind::Ident(..) => true,
            ast::Kind::Break => true,
            ast::Kind::Return => true,
            _ => false,
        }
    }
}
