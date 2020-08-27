use crate::ast;
use crate::ast::{
    BinOp, CallFn, CallInstanceFn, Colon, Eq, ExprAwait, ExprBinary, ExprField, ExprFieldAccess,
    ExprFor, ExprIndexGet, ExprIndexSet, ExprLoop, ExprTry, ExprWhile, Label, LitUnit, OpenParen,
    Path,
};
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::token::{Delimiter, Kind, Token};
use crate::traits::{Parse, Peek};
use runestick::unit::Span;
use std::ops;

/// Indicator that an expression should be parsed with an eager brace.
#[derive(Debug, Clone, Copy)]
pub(super) struct EagerBrace(pub(super) bool);

impl ops::Deref for EagerBrace {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Indicates if field accesses should be parsed or not.
#[derive(Debug, Clone, Copy)]
pub(super) struct FieldAccess(pub(super) bool);

impl ops::Deref for FieldAccess {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A rune expression.
#[derive(Debug, Clone)]
pub enum Expr {
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
    /// An empty expression.
    Ident(ast::Ident),
    /// An path expression.
    Path(ast::Path),
    /// A function call,
    CallFn(ast::CallFn),
    /// An instance function call,
    CallInstanceFn(ast::CallInstanceFn),
    /// A field access on an expression.
    ExprFieldAccess(ast::ExprFieldAccess),
    /// A unit expression.
    LitUnit(ast::LitUnit),
    /// A boolean literal.
    LitBool(ast::LitBool),
    /// A char literal.
    LitChar(ast::LitChar),
    /// A byte literal.
    LitByte(ast::LitByte),
    /// A literal number expression.
    LitNumber(ast::LitNumber),
    /// A literal string expression.
    LitStr(ast::LitStr),
    /// A literal byte string expression.
    LitByteStr(ast::LitByteStr),
    /// A literal string expression.
    LitTemplate(ast::LitTemplate),
    /// A literal vector declaration.
    LitVec(ast::LitVec),
    /// A literal object declaration.
    LitObject(ast::LitObject),
    /// A literal tuple declaration.
    LitTuple(ast::LitTuple),
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
}

impl Expr {
    /// Test if the expression implicitly evaluates to nothing.
    pub fn produces_nothing(&self) -> bool {
        match self {
            Self::ExprWhile(..) => true,
            Self::ExprLoop(..) => true,
            Self::ExprFor(..) => true,
            Self::ExprLet(..) => true,
            Self::ExprIndexSet(..) => true,
            Self::ExprIf(expr_if) => expr_if.produces_nothing(),
            Self::ExprGroup(expr_group) => expr_group.produces_nothing(),
            Self::ExprBreak(..) => true,
            Self::ExprBinary(expr_binary) => expr_binary.produces_nothing(),
            Self::ExprBlock(expr_block) => expr_block.produces_nothing(),
            Self::ExprReturn(..) => true,
            _ => false,
        }
    }

    /// Get the span of the expression.
    pub fn span(&self) -> Span {
        match self {
            Self::ExprWhile(expr) => expr.span(),
            Self::ExprLoop(expr) => expr.span(),
            Self::ExprFor(expr) => expr.span(),
            Self::ExprLet(expr) => expr.span(),
            Self::ExprIndexSet(expr) => expr.span(),
            Self::ExprIf(expr) => expr.span(),
            Self::ExprMatch(expr) => expr.span(),
            Self::Ident(expr) => expr.span(),
            Self::Path(path) => path.span(),
            Self::CallFn(expr) => expr.span(),
            Self::CallInstanceFn(expr) => expr.span(),
            Self::ExprFieldAccess(expr) => expr.span(),
            Self::LitUnit(unit) => unit.span(),
            Self::LitBool(b) => b.span(),
            Self::LitVec(expr) => expr.span(),
            Self::LitObject(expr) => expr.span(),
            Self::LitTuple(expr) => expr.span(),
            Self::LitNumber(expr) => expr.span(),
            Self::LitByte(expr) => expr.span(),
            Self::LitChar(expr) => expr.span(),
            Self::LitStr(expr) => expr.span(),
            Self::LitByteStr(expr) => expr.span(),
            Self::LitTemplate(expr) => expr.span(),
            Self::ExprGroup(expr) => expr.span(),
            Self::ExprUnary(expr) => expr.span(),
            Self::ExprBinary(expr) => expr.span(),
            Self::ExprIndexGet(expr) => expr.span(),
            Self::ExprBreak(b) => b.span(),
            Self::ExprBlock(b) => b.span(),
            Self::ExprReturn(ret) => ret.span(),
            Self::ExprAwait(ret) => ret.span(),
            Self::ExprTry(ret) => ret.span(),
            Self::ExprSelect(ret) => ret.span(),
        }
    }

    /// Test if the entire expression is constant.
    pub fn is_const(&self) -> bool {
        match self {
            Expr::ExprBinary(binary) => binary.is_const(),
            Expr::LitUnit(..) => true,
            Expr::LitBool(..) => true,
            Expr::LitByte(..) => true,
            Expr::LitChar(..) => true,
            Expr::LitNumber(..) => true,
            Expr::LitStr(..) => true,
            Expr::LitByteStr(..) => true,
            Expr::LitVec(vec) => vec.is_const(),
            Expr::LitObject(object) => object.is_const(),
            Expr::LitTuple(tuple) => tuple.is_const(),
            Expr::ExprBlock(b) => b.is_const(),
            _ => false,
        }
    }

    /// Parse an expression without an eager brace.
    ///
    /// This is used to solve a syntax ambiguity when parsing expressions that
    /// are arguments to statements immediately followed by blocks. Like `if`,
    /// `while`, and `match`.
    pub(super) fn parse_without_eager_brace(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Self::parse_full(parser, EagerBrace(false), FieldAccess(true))
    }

    /// Full, configurable parsing of an expression.
    pub(super) fn parse_full(
        parser: &mut Parser<'_>,
        eager_brace: EagerBrace,
        field_access: FieldAccess,
    ) -> Result<Self, ParseError> {
        let lhs = Self::parse_primary(parser, eager_brace, field_access)?;
        Ok(Self::parse_expr_binary(parser, lhs, 0, eager_brace)?)
    }

    /// Parse expressions that start with an identifier.
    pub(super) fn parse_ident_start(
        parser: &mut Parser<'_>,
        eager_brace: EagerBrace,
    ) -> Result<Self, ParseError> {
        let path = parser.parse::<Path>()?;

        if *eager_brace && parser.peek::<ast::OpenBrace>()? {
            let ident = ast::LitObjectIdent::Named(path);

            return Ok(Self::LitObject(ast::LitObject::parse_with_ident(
                parser, ident,
            )?));
        }

        if !parser.peek::<OpenParen>()? {
            if path.rest.is_empty() {
                return Ok(Self::Ident(path.first));
            }

            return Ok(Self::Path(path));
        }

        Ok(Self::CallFn(CallFn {
            name: path,
            args: parser.parse()?,
        }))
    }

    /// Parsing something that opens with a parenthesis.
    pub fn parse_open_paren(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        if parser.peek::<LitUnit>()? {
            return Ok(Self::LitUnit(parser.parse()?));
        }

        let open = parser.parse::<ast::OpenParen>()?;
        let expr = ast::Expr::parse_full(parser, EagerBrace(true), FieldAccess(true))?;

        if parser.peek::<ast::CloseParen>()? {
            return Ok(Expr::ExprGroup(ast::ExprGroup {
                open,
                expr: Box::new(expr),
                close: parser.parse()?,
            }));
        }

        let lit_tuple = ast::LitTuple::parse_from_first_expr(parser, open, expr)?;
        Ok(Expr::LitTuple(lit_tuple))
    }

    /// Parse a single expression value.
    pub(super) fn parse_primary(
        parser: &mut Parser<'_>,
        eager_brace: EagerBrace,
        field_access: FieldAccess,
    ) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        let expr = match token.kind {
            Kind::Select => Self::ExprSelect(parser.parse()?),
            Kind::Label => {
                let label = Some((parser.parse::<Label>()?, parser.parse::<Colon>()?));
                let token = parser.token_peek_eof()?;

                return Ok(match token.kind {
                    Kind::While => Self::ExprWhile(ExprWhile::parse_with_label(parser, label)?),
                    Kind::Loop => Self::ExprLoop(ExprLoop::parse_with_label(parser, label)?),
                    Kind::For => Self::ExprFor(ExprFor::parse_with_label(parser, label)?),
                    _ => {
                        return Err(ParseError::ExpectedLoop {
                            actual: token.kind,
                            span: token.span,
                        });
                    }
                });
            }
            Kind::Hash => Self::LitObject(parser.parse()?),
            Kind::Bang | Kind::Ampersand | Kind::Mul => Self::ExprUnary(parser.parse()?),
            Kind::While => Self::ExprWhile(parser.parse()?),
            Kind::Loop => Self::ExprLoop(parser.parse()?),
            Kind::For => Self::ExprFor(parser.parse()?),
            Kind::Let => Self::ExprLet(parser.parse()?),
            Kind::If => Self::ExprIf(parser.parse()?),
            Kind::Match => Self::ExprMatch(parser.parse()?),
            Kind::LitNumber { .. } => Self::LitNumber(parser.parse()?),
            Kind::LitChar { .. } => Self::LitChar(parser.parse()?),
            Kind::LitByte { .. } => Self::LitByte(parser.parse()?),
            Kind::LitStr { .. } => Self::LitStr(parser.parse()?),
            Kind::LitByteStr { .. } => Self::LitByteStr(parser.parse()?),
            Kind::LitTemplate { .. } => Self::LitTemplate(parser.parse()?),
            Kind::Open(Delimiter::Parenthesis) => Self::parse_open_paren(parser)?,
            Kind::Open(Delimiter::Bracket) => Self::LitVec(parser.parse()?),
            Kind::Open(Delimiter::Brace) => Self::ExprBlock(parser.parse()?),
            Kind::True | Kind::False => Self::LitBool(parser.parse()?),
            Kind::Ident => Self::parse_ident_start(parser, eager_brace)?,
            Kind::Break => Self::ExprBreak(parser.parse()?),
            Kind::Return => Self::ExprReturn(parser.parse()?),
            _ => {
                return Err(ParseError::ExpectedExpr {
                    actual: token.kind,
                    span: token.span,
                })
            }
        };

        if !*field_access {
            return Ok(expr);
        }

        Ok(Self::parse_field_access(parser, expr)?)
    }

    /// Parse an expression chain.
    fn parse_field_access(parser: &mut Parser<'_>, mut expr: Self) -> Result<Self, ParseError> {
        while let Some(token) = parser.token_peek()? {
            match token.kind {
                Kind::Open(Delimiter::Bracket) => {
                    let index_get = ExprIndexGet {
                        target: Box::new(expr),
                        open: parser.parse()?,
                        index: Box::new(parser.parse()?),
                        close: parser.parse()?,
                    };

                    if parser.peek::<Eq>()? {
                        return Ok(Self::ExprIndexSet(ExprIndexSet {
                            target: index_get.target,
                            open: index_get.open,
                            index: index_get.index,
                            close: index_get.close,
                            eq: parser.parse()?,
                            value: Box::new(parser.parse()?),
                        }));
                    }

                    expr = Self::ExprIndexGet(index_get);
                }
                Kind::Try => {
                    expr = Expr::ExprTry(ExprTry {
                        expr: Box::new(expr),
                        try_: parser.parse()?,
                    });
                }
                Kind::Dot => {
                    let dot = parser.parse()?;
                    let token = parser.token_peek()?;

                    if let Some(token) = token {
                        match token.kind {
                            Kind::Await => {
                                expr = Expr::ExprAwait(ExprAwait {
                                    expr: Box::new(expr),
                                    dot,
                                    await_: parser.parse()?,
                                });

                                continue;
                            }
                            _ => (),
                        }
                    }

                    let next = Expr::parse_primary(parser, EagerBrace(false), FieldAccess(false))?;

                    let span = match next {
                        Expr::CallFn(call_fn) => {
                            let span = call_fn.span();

                            if let Some(name) = call_fn.name.try_into_ident() {
                                expr = Expr::CallInstanceFn(CallInstanceFn {
                                    instance: Box::new(expr),
                                    dot,
                                    name,
                                    args: call_fn.args,
                                });

                                continue;
                            };

                            span
                        }
                        Expr::Ident(ident) => {
                            expr = Expr::ExprFieldAccess(ExprFieldAccess {
                                expr: Box::new(expr),
                                dot,
                                expr_field: ExprField::Ident(ident),
                            });

                            continue;
                        }
                        Expr::LitNumber(n) => {
                            expr = Expr::ExprFieldAccess(ExprFieldAccess {
                                expr: Box::new(expr),
                                dot,
                                expr_field: ExprField::LitNumber(n),
                            });

                            continue;
                        }
                        other => other.span(),
                    };

                    return Err(ParseError::UnsupportedFieldAccess { span });
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse a binary expression.
    fn parse_expr_binary(
        parser: &mut Parser<'_>,
        mut lhs: Self,
        min_precedence: usize,
        eager_brace: EagerBrace,
    ) -> Result<Self, ParseError> {
        let mut lookahead_tok = parser.token_peek_pair()?;

        loop {
            let lookahead = lookahead_tok.and_then(BinOp::from_token);

            let (op, token) = match lookahead {
                Some((op, token)) if op.precedence() >= min_precedence => (op, token),
                _ => break,
            };

            for _ in 0..op.advance() {
                parser.token_next()?;
            }

            let mut rhs = Self::parse_primary(parser, eager_brace, FieldAccess(true))?;

            lookahead_tok = parser.token_peek_pair()?;

            loop {
                let (lh, _) = match lookahead_tok.and_then(BinOp::from_token) {
                    Some((lh, _)) if lh.precedence() > op.precedence() => (lh, token),
                    Some((lh, _)) if lh.precedence() == op.precedence() && !lh.is_assoc(op) => {
                        return Err(ParseError::PrecedenceGroupRequired {
                            span: lhs.span().join(rhs.span()),
                        });
                    }
                    _ => break,
                };

                rhs = Self::parse_expr_binary(parser, rhs, lh.precedence(), eager_brace)?;
                lookahead_tok = parser.token_peek_pair()?;
            }

            lhs = Expr::ExprBinary(ExprBinary {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
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
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::Expr>("foo[\"foo\"]").unwrap();
/// parse_all::<ast::Expr>("foo.bar()").unwrap();
/// parse_all::<ast::Expr>("var()").unwrap();
/// parse_all::<ast::Expr>("var").unwrap();
/// parse_all::<ast::Expr>("42").unwrap();
/// parse_all::<ast::Expr>("1 + 2 / 3 - 4 * 1").unwrap();
/// parse_all::<ast::Expr>("foo[\"bar\"]").unwrap();
/// parse_all::<ast::Expr>("let var = 42").unwrap();
/// parse_all::<ast::Expr>("let var = \"foo bar\"").unwrap();
/// parse_all::<ast::Expr>("var[\"foo\"] = \"bar\"").unwrap();
/// parse_all::<ast::Expr>("let var = objects[\"foo\"] + 1").unwrap();
/// parse_all::<ast::Expr>("var = 42").unwrap();
///
/// let expr = parse_all::<ast::Expr>(r#"
///     if 1 { } else { if 2 { } else { } }
/// "#).unwrap();
///
/// if let ast::Expr::ExprIf(..) = expr.item {
/// } else {
///     panic!("not an if statement");
/// }
///
/// // Chained function calls.
/// parse_all::<ast::Expr>("foo.bar.baz()").unwrap();
/// parse_all::<ast::Expr>("foo[0][1][2]").unwrap();
/// parse_all::<ast::Expr>("foo.bar()[0].baz()[1]").unwrap();
///
/// parse_all::<ast::Expr>("42 is int::int").unwrap();
/// ```
impl Parse for Expr {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Self::parse_full(parser, EagerBrace(true), FieldAccess(true))
    }
}

impl Peek for Expr {
    fn peek(t1: Option<Token>, t2: Option<Token>) -> bool {
        let t1 = match t1 {
            Some(t1) => t1,
            None => return false,
        };

        match t1.kind {
            Kind::Select => true,
            Kind::Label => matches!(t2.map(|t| t.kind), Some(Kind::Colon)),
            Kind::Hash => true,
            Kind::Await => true,
            Kind::Bang | Kind::Ampersand | Kind::Mul => true,
            Kind::While => true,
            Kind::Loop => true,
            Kind::For => true,
            Kind::Let => true,
            Kind::If => true,
            Kind::LitNumber { .. } => true,
            Kind::LitChar { .. } => true,
            Kind::LitByte { .. } => true,
            Kind::LitStr { .. } => true,
            Kind::LitByteStr { .. } => true,
            Kind::LitTemplate { .. } => true,
            Kind::Open(Delimiter::Parenthesis) => true,
            Kind::Open(Delimiter::Bracket) => true,
            Kind::Open(Delimiter::Brace) => true,
            Kind::True | Kind::False => true,
            Kind::Ident => true,
            Kind::Break => true,
            Kind::Return => true,
            _ => false,
        }
    }
}
