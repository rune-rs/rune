use crate::ast;
use crate::{
    OptionSpanned as _, Parse, ParseError, ParseErrorKind, Parser, Peek, Peeker, Spanned, ToTokens,
};
use std::mem::take;
use std::ops;

/// Indicator that an expression should be parsed with an eager brace.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct EagerBrace(pub(crate) bool);

impl ops::Deref for EagerBrace {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Indicator that an expression should be parsed as an eager binary expression.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EagerBinary(pub(crate) bool);

impl ops::Deref for EagerBinary {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Indicates if an expression can be called. By default, this depends on if the
/// expression is a block expression (no) or not (yes). This allows the caller
/// to contextually override that behavior.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Callable(pub(crate) bool);

impl ops::Deref for Callable {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for Callable {
    fn default() -> Self {
        Self(true)
    }
}

/// A rune expression.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum Expr {
    /// An path expression.
    Path(Box<ast::Path>),
    /// A declaration.
    Item(Box<ast::Item>),
    /// An assign expression.
    Assign(Box<ast::ExprAssign>),
    /// A while loop.
    While(Box<ast::ExprWhile>),
    /// An unconditional loop.
    Loop(Box<ast::ExprLoop>),
    /// An for loop.
    For(Box<ast::ExprFor>),
    /// A let expression.
    Let(Box<ast::ExprLet>),
    /// An if expression.
    If(Box<ast::ExprIf>),
    /// An match expression.
    Match(Box<ast::ExprMatch>),
    /// A function call,
    Call(Box<ast::ExprCall>),
    /// A field access on an expression.
    FieldAccess(Box<ast::ExprFieldAccess>),
    /// A grouped expression.
    Group(Box<ast::ExprGroup>),
    /// A binary expression.
    Binary(Box<ast::ExprBinary>),
    /// A unary expression.
    Unary(Box<ast::ExprUnary>),
    /// An index set operation.
    Index(Box<ast::ExprIndex>),
    /// A break expression.
    Break(Box<ast::ExprBreak>),
    /// A continue expression.
    Continue(Box<ast::ExprContinue>),
    /// A yield expression.
    Yield(Box<ast::ExprYield>),
    /// A block as an expression.
    Block(Box<ast::ExprBlock>),
    /// A return statement.
    Return(Box<ast::ExprReturn>),
    /// An await expression.
    Await(Box<ast::ExprAwait>),
    /// Try expression.
    Try(Box<ast::ExprTry>),
    /// A select expression.
    Select(Box<ast::ExprSelect>),
    /// A closure expression.
    Closure(Box<ast::ExprClosure>),
    /// A literal expression.
    Lit(Box<ast::ExprLit>),
    /// Force a specific semi-colon policy.
    ForceSemi(Box<ast::ForceSemi>),
    /// A macro call,
    MacroCall(Box<ast::MacroCall>),
    /// An object literal
    Object(Box<ast::ExprObject>),
    /// A tuple literal
    Tuple(Box<ast::ExprTuple>),
    /// A vec literal
    Vec(Box<ast::ExprVec>),
    /// A range expression.
    Range(Box<ast::ExprRange>),
}

impl Expr {
    /// Indicates if an expression needs a semicolon or must be last in a block.
    pub fn needs_semi(&self) -> bool {
        match self {
            Self::While(_) => false,
            Self::Loop(_) => false,
            Self::For(_) => false,
            Self::If(_) => false,
            Self::Match(_) => false,
            Self::Block(_) => false,
            Self::Select(_) => false,
            Self::MacroCall(macro_call) => macro_call.needs_semi(),
            Self::ForceSemi(force_semi) => force_semi.needs_semi,
            _ => true,
        }
    }

    /// Indicates if an expression is callable unless it's permitted by an
    /// override.
    pub fn is_callable(&self, callable: bool) -> bool {
        match self {
            Self::While(_) => false,
            Self::Loop(_) => callable,
            Self::For(_) => false,
            Self::If(_) => callable,
            Self::Match(_) => callable,
            Self::Select(_) => callable,
            Self::ForceSemi(expr) => expr.expr.is_callable(callable),
            _ => true,
        }
    }

    /// Take the attributes from the expression.
    pub fn take_attributes(&mut self) -> Vec<ast::Attribute> {
        match self {
            Self::Path(_) => Vec::new(),
            Self::Item(item) => item.take_attributes(),
            Self::Break(expr) => take(&mut expr.attributes),
            Self::Continue(expr) => take(&mut expr.attributes),
            Self::Yield(expr) => take(&mut expr.attributes),
            Self::Block(expr) => take(&mut expr.attributes),
            Self::Return(expr) => take(&mut expr.attributes),
            Self::Closure(expr) => take(&mut expr.attributes),
            Self::Match(expr) => take(&mut expr.attributes),
            Self::While(expr) => take(&mut expr.attributes),
            Self::Loop(expr) => take(&mut expr.attributes),
            Self::For(expr) => take(&mut expr.attributes),
            Self::Let(expr) => take(&mut expr.attributes),
            Self::If(expr) => take(&mut expr.attributes),
            Self::Select(expr) => take(&mut expr.attributes),
            Self::Lit(expr) => take(&mut expr.attributes),
            Self::Assign(expr) => take(&mut expr.attributes),
            Self::Binary(expr) => take(&mut expr.attributes),
            Self::Call(expr) => take(&mut expr.attributes),
            Self::FieldAccess(expr) => take(&mut expr.attributes),
            Self::Group(expr) => take(&mut expr.attributes),
            Self::Unary(expr) => take(&mut expr.attributes),
            Self::Index(expr) => take(&mut expr.attributes),
            Self::Await(expr) => take(&mut expr.attributes),
            Self::Try(expr) => take(&mut expr.attributes),
            Self::ForceSemi(expr) => expr.expr.take_attributes(),
            Self::Object(expr) => take(&mut expr.attributes),
            Self::Range(expr) => take(&mut expr.attributes),
            Self::Vec(expr) => take(&mut expr.attributes),
            Self::Tuple(expr) => take(&mut expr.attributes),
            Self::MacroCall(expr) => take(&mut expr.attributes),
        }
    }

    /// Access the attributes of the expression.
    pub fn attributes(&self) -> &[ast::Attribute] {
        match self {
            Self::Path(_) => &[],
            Self::Item(expr) => expr.attributes(),
            Self::Break(expr) => &expr.attributes,
            Self::Continue(expr) => &expr.attributes,
            Self::Yield(expr) => &expr.attributes,
            Self::Block(expr) => &expr.attributes,
            Self::Return(expr) => &expr.attributes,
            Self::Closure(expr) => &expr.attributes,
            Self::Match(expr) => &expr.attributes,
            Self::While(expr) => &expr.attributes,
            Self::Loop(expr) => &expr.attributes,
            Self::For(expr) => &expr.attributes,
            Self::Let(expr) => &expr.attributes,
            Self::If(expr) => &expr.attributes,
            Self::Select(expr) => &expr.attributes,
            Self::Lit(expr) => &expr.attributes,
            Self::Assign(expr) => &expr.attributes,
            Self::Binary(expr) => &expr.attributes,
            Self::Call(expr) => &expr.attributes,
            Self::FieldAccess(expr) => &expr.attributes,
            Self::Group(expr) => &expr.attributes,
            Self::Unary(expr) => &expr.attributes,
            Self::Index(expr) => &expr.attributes,
            Self::Await(expr) => &expr.attributes,
            Self::Try(expr) => &expr.attributes,
            Self::ForceSemi(expr) => expr.expr.attributes(),
            Self::MacroCall(expr) => &expr.attributes,
            Self::Object(expr) => &expr.attributes,
            Self::Range(expr) => &expr.attributes,
            Self::Tuple(expr) => &expr.attributes,
            Self::Vec(expr) => &expr.attributes,
        }
    }

    /// Check if this expression is a literal expression.
    ///
    /// There are exactly two kinds of literal expressions:
    /// * Ones that are ExprLit
    /// * Unary expressions which are the negate operation.
    pub fn is_lit(&self) -> bool {
        match self {
            Self::Lit(..) => return true,
            Self::Unary(expr_unary) => {
                if let Self::Lit(expr) = &expr_unary.expr {
                    return matches!(
                        expr.as_ref(),
                        ast::ExprLit {
                            lit: ast::Lit::Number(..),
                            ..
                        }
                    );
                }
            }
            _ => (),
        }

        false
    }

    /// Parse an expression without an eager brace.
    ///
    /// This is used to solve a syntax ambiguity when parsing expressions that
    /// are arguments to statements immediately followed by blocks. Like `if`,
    /// `while`, and `match`.
    pub(crate) fn parse_without_eager_brace(p: &mut Parser<'_>) -> Result<Self, ParseError> {
        Self::parse_with(p, EagerBrace(false), EagerBinary(true), Callable(true))
    }

    /// ull, configurable parsing of an expression.F
    pub(crate) fn parse_with(
        p: &mut Parser<'_>,
        eager_brace: EagerBrace,
        eager_binary: EagerBinary,
        callable: Callable,
    ) -> Result<Self, ParseError> {
        let mut attributes = p.parse()?;

        let expr = Self::parse_base(p, &mut attributes, eager_brace)?;
        let expr = Self::parse_chain(p, expr, callable)?;

        let expr = if *eager_binary {
            Self::parse_binary(p, expr, 0, eager_brace)?
        } else {
            expr
        };

        if let Some(span) = attributes.option_span() {
            return Err(ParseError::unsupported(span, "attributes"));
        }

        Ok(expr)
    }

    /// Parse expressions that start with an identifier.
    pub(crate) fn parse_with_meta_path(
        p: &mut Parser<'_>,
        attributes: &mut Vec<ast::Attribute>,
        path: ast::Path,
        eager_brace: EagerBrace,
    ) -> Result<Self, ParseError> {
        if *eager_brace && p.peek::<T!['{']>()? {
            let ident = ast::ObjectIdent::Named(path);

            return Ok(Self::Object(Box::new(ast::ExprObject::parse_with_meta(
                p,
                take(attributes),
                ident,
            )?)));
        }

        if p.peek::<T![!]>()? {
            return Ok(Self::MacroCall(Box::new(
                ast::MacroCall::parse_with_meta_path(p, std::mem::take(attributes), path)?,
            )));
        }

        Ok(Self::Path(Box::new(path)))
    }

    /// Parsing something that opens with a parenthesis.
    pub fn parse_open_paren(
        p: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        // Special case: empty tuple.
        if let (K!['('], K![')']) = (p.nth(0)?, p.nth(1)?) {
            return Ok(Self::Tuple(Box::new(ast::ExprTuple::parse_with_meta(
                p, attributes,
            )?)));
        }

        let open = p.parse::<T!['(']>()?;
        let expr = p.parse::<Self>()?;

        if p.peek::<T![')']>()? {
            return Ok(Self::Group(Box::new(ast::ExprGroup {
                attributes,
                open,
                expr,
                close: p.parse()?,
            })));
        }

        Ok(Self::Tuple(Box::new(
            ast::ExprTuple::parse_from_first_expr(p, attributes, open, expr)?,
        )))
    }

    pub(crate) fn parse_with_meta(
        p: &mut Parser<'_>,
        attributes: &mut Vec<ast::Attribute>,
        path: Option<ast::Path>,
        callable: Callable,
    ) -> Result<Self, ParseError> {
        let lhs = if let Some(path) = path {
            Self::parse_with_meta_path(p, attributes, path, EagerBrace(true))?
        } else {
            Self::parse_base(p, attributes, EagerBrace(true))?
        };

        let lhs = Self::parse_chain(p, lhs, callable)?;
        Self::parse_binary(p, lhs, 0, EagerBrace(true))
    }

    /// Parse a basic expression.
    fn parse_base(
        p: &mut Parser<'_>,
        attributes: &mut Vec<ast::Attribute>,
        eager_brace: EagerBrace,
    ) -> Result<Self, ParseError> {
        if let Some(path) = p.parse::<Option<ast::Path>>()? {
            return Self::parse_with_meta_path(p, attributes, path, eager_brace);
        }

        if ast::Lit::peek_in_expr(p.peeker()) {
            return Ok(Self::Lit(Box::new(ast::ExprLit::parse_with_meta(
                p,
                take(attributes),
            )?)));
        }

        let mut label = p.parse::<Option<(ast::Label, T![:])>>()?;
        let mut async_token = p.parse::<Option<T![async]>>()?;
        let mut const_token = p.parse::<Option<T![const]>>()?;
        let mut move_token = p.parse::<Option<T![move]>>()?;

        let expr = match p.nth(0)? {
            K![..] => {
                let limits = ast::ExprRangeLimits::HalfOpen(p.parse()?);
                Self::parse_range(p, take(attributes), None, limits)?
            }
            K![..=] => {
                let limits = ast::ExprRangeLimits::Closed(p.parse()?);
                Self::parse_range(p, take(attributes), None, limits)?
            }
            K![#] => {
                let ident = ast::ObjectIdent::Anonymous(p.parse()?);

                Self::Object(Box::new(ast::ExprObject::parse_with_meta(
                    p,
                    take(attributes),
                    ident,
                )?))
            }
            K![||] | K![|] => Self::Closure(Box::new(ast::ExprClosure::parse_with_meta(
                p,
                take(attributes),
                take(&mut async_token),
                take(&mut move_token),
            )?)),
            K![select] => Self::Select(Box::new(ast::ExprSelect::parse_with_attributes(
                p,
                take(attributes),
            )?)),
            K![!] | K![-] | K![&] | K![*] => Self::Unary(Box::new(
                ast::ExprUnary::parse_with_meta(p, take(attributes), eager_brace)?,
            )),
            K![while] => Self::While(Box::new(ast::ExprWhile::parse_with_meta(
                p,
                take(attributes),
                take(&mut label),
            )?)),
            K![loop] => Self::Loop(Box::new(ast::ExprLoop::parse_with_meta(
                p,
                take(attributes),
                take(&mut label),
            )?)),
            K![for] => Self::For(Box::new(ast::ExprFor::parse_with_meta(
                p,
                take(attributes),
                take(&mut label),
            )?)),
            K![let] => Self::Let(Box::new(ast::ExprLet::parse_with_meta(
                p,
                take(attributes),
            )?)),
            K![if] => Self::If(Box::new(ast::ExprIf::parse_with_meta(p, take(attributes))?)),
            K![match] => Self::Match(Box::new(ast::ExprMatch::parse_with_attributes(
                p,
                take(attributes),
            )?)),
            K!['['] => Self::Vec(Box::new(ast::ExprVec::parse_with_meta(
                p,
                take(attributes),
            )?)),
            K!['('] => Self::parse_open_paren(p, take(attributes))?,
            K!['{'] => Self::Block(Box::new(ast::ExprBlock::parse_with_meta(
                p,
                take(attributes),
                take(&mut async_token),
                take(&mut const_token),
                take(&mut move_token),
            )?)),
            K![break] => Self::Break(Box::new(ast::ExprBreak::parse_with_meta(
                p,
                take(attributes),
            )?)),
            K![continue] => Self::Continue(Box::new(ast::ExprContinue::parse_with_meta(
                p,
                take(attributes),
            )?)),
            K![yield] => Self::Yield(Box::new(ast::ExprYield::parse_with_meta(
                p,
                take(attributes),
            )?)),
            K![return] => Self::Return(Box::new(ast::ExprReturn::parse_with_meta(
                p,
                take(attributes),
            )?)),
            _ => {
                return Err(ParseError::expected(&p.tok_at(0)?, "expression"));
            }
        };

        if let Some(span) = label.option_span() {
            return Err(ParseError::unsupported(span, "label"));
        }

        if let Some(span) = async_token.option_span() {
            return Err(ParseError::unsupported(span, "async modifier"));
        }

        if let Some(span) = const_token.option_span() {
            return Err(ParseError::unsupported(span, "const modifier"));
        }

        if let Some(span) = move_token.option_span() {
            return Err(ParseError::unsupported(span, "move modifier"));
        }

        Ok(expr)
    }

    /// Parse the tail-end of a range.
    fn parse_range(
        p: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
        from: Option<Self>,
        limits: ast::ExprRangeLimits,
    ) -> Result<Self, ParseError> {
        let to = if Self::peek(p.peeker()) {
            Some(Self::parse_with(
                p,
                EagerBrace(true),
                EagerBinary(true),
                Callable(true),
            )?)
        } else {
            None
        };

        Ok(Self::Range(Box::new(ast::ExprRange {
            attributes,
            from,
            limits,
            to,
        })))
    }

    /// Parse an expression chain.
    fn parse_chain(
        p: &mut Parser<'_>,
        mut expr: Self,
        callable: Callable,
    ) -> Result<Self, ParseError> {
        while !p.is_eof()? {
            let is_callable = expr.is_callable(*callable);

            match p.nth(0)? {
                K!['['] if is_callable => {
                    expr = Self::Index(Box::new(ast::ExprIndex {
                        attributes: expr.take_attributes(),
                        target: expr,
                        open: p.parse()?,
                        index: p.parse()?,
                        close: p.parse()?,
                    }));
                }
                // Chained function call.
                K!['('] if is_callable => {
                    let args = p.parse::<ast::Parenthesized<Self, T![,]>>()?;

                    expr = Self::Call(Box::new(ast::ExprCall {
                        id: Default::default(),
                        attributes: expr.take_attributes(),
                        expr,
                        args,
                    }));
                }
                K![?] => {
                    expr = Self::Try(Box::new(ast::ExprTry {
                        attributes: expr.take_attributes(),
                        expr,
                        try_token: p.parse()?,
                    }));
                }
                K![=] => {
                    let eq = p.parse()?;
                    let rhs =
                        Self::parse_with(p, EagerBrace(true), EagerBinary(true), Callable(true))?;

                    expr = Self::Assign(Box::new(ast::ExprAssign {
                        attributes: expr.take_attributes(),
                        lhs: expr,
                        eq,
                        rhs,
                    }));
                }
                K![.] => {
                    match p.nth(1)? {
                        // <expr>.await
                        K![await] => {
                            expr = Self::Await(Box::new(ast::ExprAwait {
                                attributes: expr.take_attributes(),
                                expr,
                                dot: p.parse()?,
                                await_token: p.parse()?,
                            }));
                        }
                        // <expr>.field
                        K![ident] => {
                            expr = Self::FieldAccess(Box::new(ast::ExprFieldAccess {
                                attributes: expr.take_attributes(),
                                expr,
                                dot: p.parse()?,
                                expr_field: ast::ExprField::Path(p.parse()?),
                            }));
                        }
                        // tuple access: <expr>.<number>
                        K![number] => {
                            expr = Self::FieldAccess(Box::new(ast::ExprFieldAccess {
                                attributes: expr.take_attributes(),
                                expr,
                                dot: p.parse()?,
                                expr_field: ast::ExprField::LitNumber(p.parse()?),
                            }));
                        }
                        _ => {
                            return Err(ParseError::new(
                                p.span(0..1),
                                ParseErrorKind::BadFieldAccess,
                            ));
                        }
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_binary_rhs(
        p: &mut Parser<'_>,
        lhs: &Self,
        eager_brace: EagerBrace,
        op: ast::BinOp,
        lookahead_tok: &mut Option<ast::BinOp>,
    ) -> Result<Self, ParseError> {
        let rhs = Self::parse_base(p, &mut vec![], eager_brace)?;
        let mut rhs = Self::parse_chain(p, rhs, Callable(true))?;

        *lookahead_tok = ast::BinOp::from_peeker(p.peeker());

        loop {
            let lh = match *lookahead_tok {
                Some(lh) if lh.precedence() > op.precedence() => lh,
                Some(lh) if lh.precedence() == op.precedence() && !op.is_assoc() => {
                    return Err(ParseError::new(
                        lhs.span().join(rhs.span()),
                        ParseErrorKind::PrecedenceGroupRequired,
                    ));
                }
                _ => break,
            };

            rhs = Self::parse_binary(p, rhs, lh.precedence(), eager_brace)?;
            *lookahead_tok = ast::BinOp::from_peeker(p.peeker());
        }

        Ok(rhs)
    }

    /// Try parse rhs in binary expression.
    fn try_parse_binary_rhs(
        p: &mut Parser<'_>,
        lhs: &Self,
        eager_brace: EagerBrace,
        op: ast::BinOp,
        lookahead_tok: &mut Option<ast::BinOp>,
    ) -> Result<Option<Self>, ParseError> {
        Ok(if Self::peek(p.peeker()) {
            Some(Self::parse_binary_rhs(
                p,
                lhs,
                eager_brace,
                op,
                lookahead_tok,
            )?)
        } else {
            *lookahead_tok = None;
            None
        })
    }

    /// Parse a binary expression.
    fn parse_binary(
        p: &mut Parser<'_>,
        mut lhs: Self,
        min_precedence: usize,
        eager_brace: EagerBrace,
    ) -> Result<Self, ParseError> {
        let mut lookahead_tok = ast::BinOp::from_peeker(p.peeker());

        loop {
            let op = match lookahead_tok {
                Some(op) if op.precedence() >= min_precedence => op,
                _ => break,
            };

            let (t1, t2) = op.advance(p)?;

            lhs = match t1.kind {
                K![..] => {
                    let to =
                        Self::try_parse_binary_rhs(p, &lhs, eager_brace, op, &mut lookahead_tok)?;

                    Self::Range(Box::new(ast::ExprRange {
                        attributes: lhs.take_attributes(),
                        from: Some(lhs),
                        limits: ast::ExprRangeLimits::HalfOpen(ast::generated::DotDot {
                            token: t1,
                        }),
                        to,
                    }))
                }
                K![..=] => {
                    let to =
                        Self::try_parse_binary_rhs(p, &lhs, eager_brace, op, &mut lookahead_tok)?;

                    Self::Range(Box::new(ast::ExprRange {
                        attributes: lhs.take_attributes(),
                        from: Some(lhs),
                        limits: ast::ExprRangeLimits::Closed(ast::generated::DotDotEq {
                            token: t1,
                        }),
                        to,
                    }))
                }
                _ => {
                    let rhs = Self::parse_binary_rhs(p, &lhs, eager_brace, op, &mut lookahead_tok)?;

                    Self::Binary(Box::new(ast::ExprBinary {
                        attributes: Vec::new(),
                        lhs,
                        t1,
                        t2,
                        rhs,
                        op,
                    }))
                }
            };
        }

        Ok(lhs)
    }

    /// Internal function to construct a literal expression.
    pub(crate) fn from_lit(lit: ast::Lit) -> Self {
        Self::Lit(Box::new(ast::ExprLit {
            attributes: Vec::new(),
            lit,
        }))
    }
}

/// Parsing a block expression.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::Expr>("()");
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
/// assert!(matches!(expr, ast::Expr::If(..)));
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
/// assert!(matches!(expr, ast::Expr::Block(b) if b.attributes.len() == 1 && b.block.statements.len() == 1));
///
/// testing::roundtrip::<ast::Expr>("#{\"foo\": b\"bar\"}");
/// testing::roundtrip::<ast::Expr>("Disco {\"never_died\": true }");
/// testing::roundtrip::<ast::Expr>("(false, 1, 'n')");
/// testing::roundtrip::<ast::Expr>("[false, 1, 'b']");
/// ```
impl Parse for Expr {
    fn parse(p: &mut Parser<'_>) -> Result<Self, ParseError> {
        Self::parse_with(p, EagerBrace(true), EagerBinary(true), Callable(true))
    }
}

impl Peek for Expr {
    fn peek(p: &mut Peeker<'_>) -> bool {
        match p.nth(0) {
            K![async] => true,
            K![self] => true,
            K![select] => true,
            K![#] => true,
            K![-] => true,
            K![!] => true,
            K![&] => true,
            K![*] => true,
            K![while] => true,
            K![loop] => true,
            K![for] => true,
            K![let] => true,
            K![if] => true,
            K![break] => true,
            K![continue] => true,
            K![return] => true,
            K![true] => true,
            K![false] => true,
            K![ident] => true,
            K![::] => true,
            K!['('] => true,
            K!['['] => true,
            K!['{'] => true,
            K![number] => true,
            K![char] => true,
            K![byte] => true,
            K![str] => true,
            K![bytestr] => true,
            K!['label] => matches!(p.nth(1), K![:]),
            K![..] => true,
            _ => false,
        }
    }
}

/// Used to parse an expression without supporting an immediate binary expression.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprWithoutBinary {
    expr: ast::Expr,
}

impl ExprWithoutBinary {
    /// Access the inner expression.
    pub fn into_expr(self) -> Expr {
        self.expr
    }
}

impl Parse for ExprWithoutBinary {
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        let expr = Expr::parse_with(
            p,
            Default::default(),
            EagerBinary(false),
            Default::default(),
        )?;

        Ok(Self { expr })
    }
}

impl Peek for ExprWithoutBinary {
    fn peek(p: &mut Peeker<'_>) -> bool {
        Expr::peek(p)
    }
}

#[cfg(test)]
mod tests {
    use crate::{ast, testing};

    #[test]
    fn test_expr_if() {
        testing::roundtrip::<ast::Expr>(r#"if true {} else {}"#);
    }

    #[test]
    fn test_expr_while() {
        testing::roundtrip::<ast::ExprWhile>(r#"while true {}"#);
    }
}
