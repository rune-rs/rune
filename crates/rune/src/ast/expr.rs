use crate::ast;
use crate::{Parse, ParseError, Parser, Peek};
use runestick::Span;
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
pub(super) struct ExprChain(pub(super) bool);

impl ops::Deref for ExprChain {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A rune expression.
#[derive(Debug, Clone)]
pub enum Expr {
    /// The `self` keyword.
    Self_(ast::Self_),
    /// An path expression.
    Path(ast::Path),
    /// A declaration.
    // large size difference between variants
    // we should box this variant.
    // https://rust-lang.github.io/rust-clippy/master/index.html#large_enum_variant
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
    /// An async block as an expression.
    ExprAsync(ast::ExprAsync),
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
}

into_tokens_enum! {
    Expr {
        Self_,
        Path,
        Item,
        ExprWhile,
        ExprLoop,
        ExprFor,
        ExprLet,
        ExprIndexSet,
        ExprIf,
        ExprMatch,
        ExprCall,
        MacroCall,
        ExprFieldAccess,
        ExprGroup,
        ExprBinary,
        ExprUnary,
        ExprIndexGet,
        ExprBreak,
        ExprYield,
        ExprBlock,
        ExprAsync,
        ExprReturn,
        ExprAwait,
        ExprTry,
        ExprSelect,
        ExprClosure,
        LitUnit,
        LitBool,
        LitChar,
        LitByte,
        LitNumber,
        LitStr,
        LitByteStr,
        LitTemplate,
        LitVec,
        LitObject,
        LitTuple
    }
}

impl Expr {
    /// Indicates if an expression needs a semicolon or must be last in a block.
    pub fn needs_semi(&self) -> bool {
        match self {
            Expr::ExprWhile(_) => false,
            Expr::ExprLoop(_) => false,
            Expr::ExprFor(_) => false,
            Expr::ExprIf(_) => false,
            Expr::ExprMatch(_) => false,
            Expr::ExprBlock(_) => false,
            Expr::ExprAsync(_) => false,
            Expr::ExprSelect(_) => false,
            _ => true,
        }
    }

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
            Self::ExprAsync(..) => false,
            Self::ExprReturn(..) => true,
            _ => false,
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

    /// Get the span of the expression.
    pub fn span(&self) -> Span {
        match self {
            Self::Self_(s) => s.span(),
            Self::Path(path) => path.span(),
            Self::Item(decl) => decl.span(),
            Self::ExprWhile(expr) => expr.span(),
            Self::ExprLoop(expr) => expr.span(),
            Self::ExprFor(expr) => expr.span(),
            Self::ExprLet(expr) => expr.span(),
            Self::ExprIndexSet(expr) => expr.span(),
            Self::ExprIf(expr) => expr.span(),
            Self::ExprMatch(expr) => expr.span(),
            Self::ExprCall(expr) => expr.span(),
            Self::MacroCall(expr) => expr.span(),
            Self::ExprFieldAccess(expr) => expr.span(),
            Self::ExprGroup(expr) => expr.span(),
            Self::ExprUnary(expr) => expr.span(),
            Self::ExprBinary(expr) => expr.span(),
            Self::ExprIndexGet(expr) => expr.span(),
            Self::ExprBreak(b) => b.span(),
            Self::ExprYield(b) => b.span(),
            Self::ExprBlock(b) => b.span(),
            Self::ExprAsync(b) => b.span(),
            Self::ExprReturn(ret) => ret.span(),
            Self::ExprAwait(ret) => ret.span(),
            Self::ExprTry(ret) => ret.span(),
            Self::ExprSelect(ret) => ret.span(),
            Self::ExprClosure(ret) => ret.span(),
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
        Self::parse_full(parser, EagerBrace(false), ExprChain(true))
    }

    /// Full, configurable parsing of an expression.
    pub(super) fn parse_full(
        parser: &mut Parser<'_>,
        eager_brace: EagerBrace,
        expr_chain: ExprChain,
    ) -> Result<Self, ParseError> {
        let lhs = Self::parse_primary(parser, eager_brace, expr_chain)?;
        Ok(Self::parse_expr_binary(parser, lhs, 0, eager_brace)?)
    }

    /// Parse expressions that start with an identifier.
    pub(super) fn parse_ident_start(
        parser: &mut Parser<'_>,
        eager_brace: EagerBrace,
    ) -> Result<Self, ParseError> {
        let path = parser.parse::<ast::Path>()?;

        if *eager_brace && parser.peek::<ast::OpenBrace>()? {
            let ident = ast::LitObjectIdent::Named(path);

            return Ok(Self::LitObject(ast::LitObject::parse_with_ident(
                parser, ident,
            )?));
        }

        if parser.peek::<ast::Bang>()? {
            return Ok(Self::MacroCall(ast::MacroCall::parse_with_path(
                parser, path,
            )?));
        }

        Ok(Self::Path(path))
    }

    /// Parsing something that opens with a parenthesis.
    pub fn parse_open_paren(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        if parser.peek::<ast::LitUnit>()? {
            return Ok(Self::LitUnit(parser.parse()?));
        }

        let open = parser.parse::<ast::OpenParen>()?;
        let expr = ast::Expr::parse_full(parser, EagerBrace(true), ExprChain(true))?;

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
        expr_chain: ExprChain,
    ) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        let expr = match token.kind {
            ast::Kind::Async => {
                let async_: ast::Async = parser.parse()?;
                let expr: Self = Self::parse_primary(parser, eager_brace, expr_chain)?;

                match expr {
                    Self::ExprClosure(expr_closure) => Self::ExprClosure(ast::ExprClosure {
                        async_: Some(async_),
                        args: expr_closure.args,
                        body: expr_closure.body,
                    }),
                    Self::ExprBlock(expr_block) => Self::ExprAsync(ast::ExprAsync {
                        async_,
                        block: expr_block.block,
                    }),
                    _ => return Err(ParseError::UnsupportedAsyncExpr { span: expr.span() }),
                }
            }
            ast::Kind::Self_ => Self::Self_(parser.parse()?),
            ast::Kind::Select => Self::ExprSelect(parser.parse()?),
            ast::Kind::PipePipe | ast::Kind::Pipe => Self::ExprClosure(parser.parse()?),
            ast::Kind::Label(..) => {
                let label = Some((parser.parse::<ast::Label>()?, parser.parse::<ast::Colon>()?));
                let token = parser.token_peek_eof()?;

                return Ok(match token.kind {
                    ast::Kind::While => {
                        Self::ExprWhile(ast::ExprWhile::parse_with_label(parser, label)?)
                    }
                    ast::Kind::Loop => {
                        Self::ExprLoop(ast::ExprLoop::parse_with_label(parser, label)?)
                    }
                    ast::Kind::For => Self::ExprFor(ast::ExprFor::parse_with_label(parser, label)?),
                    _ => {
                        return Err(ParseError::ExpectedLoop {
                            actual: token.kind,
                            span: token.span,
                        });
                    }
                });
            }
            ast::Kind::Pound => Self::LitObject(parser.parse()?),
            ast::Kind::Bang | ast::Kind::Amp | ast::Kind::Star => Self::ExprUnary(parser.parse()?),
            ast::Kind::While => Self::ExprWhile(parser.parse()?),
            ast::Kind::Loop => Self::ExprLoop(parser.parse()?),
            ast::Kind::For => Self::ExprFor(parser.parse()?),
            ast::Kind::Let => Self::ExprLet(parser.parse()?),
            ast::Kind::If => Self::ExprIf(parser.parse()?),
            ast::Kind::Match => Self::ExprMatch(parser.parse()?),
            ast::Kind::LitNumber { .. } => Self::LitNumber(parser.parse()?),
            ast::Kind::LitChar { .. } => Self::LitChar(parser.parse()?),
            ast::Kind::LitByte { .. } => Self::LitByte(parser.parse()?),
            ast::Kind::LitStr { .. } => Self::LitStr(parser.parse()?),
            ast::Kind::LitByteStr { .. } => Self::LitByteStr(parser.parse()?),
            ast::Kind::LitTemplate { .. } => Self::LitTemplate(parser.parse()?),
            ast::Kind::Open(ast::Delimiter::Parenthesis) => Self::parse_open_paren(parser)?,
            ast::Kind::Open(ast::Delimiter::Bracket) => Self::LitVec(parser.parse()?),
            ast::Kind::Open(ast::Delimiter::Brace) => Self::ExprBlock(parser.parse()?),
            ast::Kind::True | ast::Kind::False => Self::LitBool(parser.parse()?),
            ast::Kind::Ident(..) => Self::parse_ident_start(parser, eager_brace)?,
            ast::Kind::Break => Self::ExprBreak(parser.parse()?),
            ast::Kind::Yield => Self::ExprYield(parser.parse()?),
            ast::Kind::Return => Self::ExprReturn(parser.parse()?),
            _ => {
                return Err(ParseError::ExpectedExpr {
                    actual: token.kind,
                    span: token.span,
                })
            }
        };

        if !*expr_chain {
            return Ok(expr);
        }

        Ok(Self::parse_expr_chain(parser, expr)?)
    }

    /// Parse an expression chain.
    fn parse_expr_chain(parser: &mut Parser<'_>, mut expr: Self) -> Result<Self, ParseError> {
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
                    let token = parser.token_peek()?;

                    if let Some(token) = token {
                        if let ast::Kind::Await = token.kind {
                            expr = Expr::ExprAwait(ast::ExprAwait {
                                expr: Box::new(expr),
                                dot,
                                await_: parser.parse()?,
                            });

                            continue;
                        }
                    }

                    let next = Expr::parse_primary(parser, EagerBrace(false), ExprChain(false))?;

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
                        Expr::LitNumber(n) => {
                            expr = Expr::ExprFieldAccess(ast::ExprFieldAccess {
                                expr: Box::new(expr),
                                dot,
                                expr_field: ast::ExprField::LitNumber(n),
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
            let lookahead = lookahead_tok.and_then(ast::BinOp::from_token);

            let (op, t1, t2) = match lookahead {
                Some((op, t1, t2)) if op.precedence() >= min_precedence => (op, t1, t2),
                _ => break,
            };

            for _ in 0..op.advance() {
                parser.token_next()?;
            }

            let mut rhs = Self::parse_primary(parser, eager_brace, ExprChain(true))?;

            lookahead_tok = parser.token_peek_pair()?;

            loop {
                let lh = match lookahead_tok.and_then(ast::BinOp::from_token) {
                    Some((lh, _, _)) if lh.precedence() > op.precedence() => lh,
                    Some((lh, _, _)) if lh.precedence() == op.precedence() && !op.is_assoc() => {
                        return Err(ParseError::PrecedenceGroupRequired {
                            span: lhs.span().join(rhs.span()),
                        });
                    }
                    _ => break,
                };

                rhs = Self::parse_expr_binary(parser, rhs, lh.precedence(), eager_brace)?;
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
/// if let ast::Expr::ExprIf(..) = expr {
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
        Self::parse_full(parser, EagerBrace(true), ExprChain(true))
    }
}

impl Peek for Expr {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        let t1 = match t1 {
            Some(t1) => t1,
            None => return false,
        };

        match t1.kind {
            ast::Kind::Async => true,
            ast::Kind::Self_ => true,
            ast::Kind::Select => true,
            ast::Kind::Label(..) => matches!(t2.map(|t| t.kind), Some(ast::Kind::Colon)),
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
