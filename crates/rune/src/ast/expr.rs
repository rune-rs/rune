use core::mem::take;
use core::ops;

use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::Expr>("()");
    rt::<ast::Expr>("foo[\"foo\"]");
    rt::<ast::Expr>("foo.bar()");
    rt::<ast::Expr>("var()");
    rt::<ast::Expr>("var");
    rt::<ast::Expr>("42");
    rt::<ast::Expr>("1 + 2 / 3 - 4 * 1");
    rt::<ast::Expr>("foo[\"bar\"]");
    rt::<ast::Expr>("let var = 42");
    rt::<ast::Expr>("let var = \"foo bar\"");
    rt::<ast::Expr>("var[\"foo\"] = \"bar\"");
    rt::<ast::Expr>("let var = objects[\"foo\"] + 1");
    rt::<ast::Expr>("var = 42");

    let expr = rt::<ast::Expr>(
        r#"
        if 1 { } else { if 2 { } else { } }
    "#,
    );
    assert!(matches!(expr, ast::Expr::If(..)));

    // Chained function calls.
    rt::<ast::Expr>("foo.bar.baz()");
    rt::<ast::Expr>("foo[0][1][2]");
    rt::<ast::Expr>("foo.bar()[0].baz()[1]");

    rt::<ast::Expr>("42 is i64::i64");
    rt::<ast::Expr>("{ let x = 1; x }");

    let expr = rt::<ast::Expr>("#[cfg(debug_assertions)] { assert_eq(x, 32); }");
    assert!(
        matches!(expr, ast::Expr::Block(b) if b.attributes.len() == 1 && b.block.statements.len() == 1)
    );

    rt::<ast::Expr>("#{\"foo\": b\"bar\"}");
    rt::<ast::Expr>("Disco {\"never_died\": true }");
    rt::<ast::Expr>("(false, 1, 'n')");
    rt::<ast::Expr>("[false, 1, 'b']");
}

/// Indicator that an expression should be parsed with an eager brace.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EagerBrace(bool);

/// Indicates that an expression should be parsed with eager braces.
pub(crate) const EAGER_BRACE: EagerBrace = EagerBrace(true);

/// Indicates that an expression should not be parsed with eager braces. This is
/// used to solve a parsing ambiguity.
pub(crate) const NOT_EAGER_BRACE: EagerBrace = EagerBrace(false);

impl ops::Deref for EagerBrace {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Indicator that an expression should be parsed as an eager binary expression.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EagerBinary(bool);

/// Indicates that an expression should be parsed as a binary expression.
pub(crate) const EAGER_BINARY: EagerBinary = EagerBinary(true);

/// Indicates that an expression should not be parsed as a binary expression.
pub(crate) const NOT_EAGER_BINARY: EagerBinary = EagerBinary(false);

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
pub(crate) struct Callable(bool);

/// Indicates that an expression should be treated as if it could be callable.
/// Such as `foo::bar(42)`.
pub(crate) const CALLABLE: Callable = Callable(true);

/// Indicates that an expression should be treated as if it's *not* callable.
/// This is used to solve otherwise parsing ambiguities.
pub(crate) const NOT_CALLABLE: Callable = Callable(false);

impl ops::Deref for Callable {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A rune expression.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum Expr {
    /// An path expression.
    Path(ast::Path),
    /// An assign expression.
    Assign(ast::ExprAssign),
    /// A while loop.
    While(ast::ExprWhile),
    /// An unconditional loop.
    Loop(ast::ExprLoop),
    /// An for loop.
    For(ast::ExprFor),
    /// A let expression.
    Let(ast::ExprLet),
    /// An if expression.
    If(ast::ExprIf),
    /// An match expression.
    Match(ast::ExprMatch),
    /// A function call,
    Call(ast::ExprCall),
    /// A field access on an expression.
    FieldAccess(ast::ExprFieldAccess),
    /// A binary expression.
    Binary(ast::ExprBinary),
    /// A unary expression.
    Unary(ast::ExprUnary),
    /// An index set operation.
    Index(ast::ExprIndex),
    /// A break expression.
    Break(ast::ExprBreak),
    /// A continue expression.
    Continue(ast::ExprContinue),
    /// A yield expression.
    Yield(ast::ExprYield),
    /// A block as an expression.
    Block(ast::ExprBlock),
    /// A return statement.
    Return(ast::ExprReturn),
    /// An await expression.
    Await(ast::ExprAwait),
    /// Try expression.
    Try(ast::ExprTry),
    /// A select expression.
    Select(ast::ExprSelect),
    /// A closure expression.
    Closure(ast::ExprClosure),
    /// A literal expression.
    Lit(ast::ExprLit),
    /// An object literal
    Object(ast::ExprObject),
    /// A tuple literal
    Tuple(ast::ExprTuple),
    /// A vec literal
    Vec(ast::ExprVec),
    /// A range expression.
    Range(ast::ExprRange),
    /// A grouped empty expression.
    Empty(ast::ExprEmpty),
    /// A grouped expression.
    Group(ast::ExprGroup),
    /// A macro call,
    MacroCall(ast::MacroCall),
}

impl Expr {
    /// Access the attributes of the expression.
    pub(crate) fn attributes(&self) -> &[ast::Attribute] {
        match self {
            Self::Path(_) => &[],
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
            Self::Empty(expr) => &expr.attributes,
            Self::Unary(expr) => &expr.attributes,
            Self::Index(expr) => &expr.attributes,
            Self::Await(expr) => &expr.attributes,
            Self::Try(expr) => &expr.attributes,
            Self::MacroCall(expr) => &expr.attributes,
            Self::Object(expr) => &expr.attributes,
            Self::Range(expr) => &expr.attributes,
            Self::Tuple(expr) => &expr.attributes,
            Self::Vec(expr) => &expr.attributes,
        }
    }

    /// Indicates if an expression needs a semicolon or must be last in a block.
    pub(crate) fn needs_semi(&self) -> bool {
        match self {
            Self::While(_) => false,
            Self::Loop(_) => false,
            Self::For(_) => false,
            Self::If(_) => false,
            Self::Match(_) => false,
            Self::Block(_) => false,
            Self::Select(_) => false,
            Self::MacroCall(macro_call) => macro_call.needs_semi(),
            _ => true,
        }
    }

    /// Indicates if an expression is callable unless it's permitted by an
    /// override.
    pub(crate) fn is_callable(&self, callable: bool) -> bool {
        match self {
            Self::While(_) => false,
            Self::Loop(_) => callable,
            Self::For(_) => false,
            Self::If(_) => callable,
            Self::Match(_) => callable,
            Self::Select(_) => callable,
            _ => true,
        }
    }

    /// Take the attributes from the expression.
    pub(crate) fn take_attributes(&mut self) -> Vec<ast::Attribute> {
        match self {
            Self::Path(_) => Vec::new(),
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
            Self::Empty(expr) => take(&mut expr.attributes),
            Self::Unary(expr) => take(&mut expr.attributes),
            Self::Index(expr) => take(&mut expr.attributes),
            Self::Await(expr) => take(&mut expr.attributes),
            Self::Try(expr) => take(&mut expr.attributes),
            Self::Object(expr) => take(&mut expr.attributes),
            Self::Range(expr) => take(&mut expr.attributes),
            Self::Vec(expr) => take(&mut expr.attributes),
            Self::Tuple(expr) => take(&mut expr.attributes),
            Self::MacroCall(expr) => take(&mut expr.attributes),
        }
    }

    /// Check if this expression is a literal expression.
    ///
    /// There are exactly two kinds of literal expressions:
    /// * Ones that are ExprLit
    /// * Unary expressions which are the negate operation.
    pub(crate) fn is_lit(&self) -> bool {
        match self {
            Self::Lit(..) => return true,
            Self::Unary(ast::ExprUnary {
                op: ast::UnOp::Neg(..),
                expr,
                ..
            }) => {
                return matches!(
                    &**expr,
                    Self::Lit(ast::ExprLit {
                        lit: ast::Lit::Number(..),
                        ..
                    })
                );
            }
            _ => (),
        }

        false
    }

    /// Internal function to construct a literal expression.
    pub(crate) fn from_lit(lit: ast::Lit) -> Self {
        Self::Lit(ast::ExprLit {
            attributes: Vec::new(),
            lit,
        })
    }

    /// Parse an expression without an eager brace.
    ///
    /// This is used to solve a syntax ambiguity when parsing expressions that
    /// are arguments to statements immediately followed by blocks. Like `if`,
    /// `while`, and `match`.
    pub(crate) fn parse_without_eager_brace(p: &mut Parser<'_>) -> Result<Self> {
        Self::parse_with(p, NOT_EAGER_BRACE, EAGER_BINARY, CALLABLE)
    }

    /// Helper to perform a parse with the given meta.
    pub(crate) fn parse_with_meta(
        p: &mut Parser<'_>,
        attributes: &mut Vec<ast::Attribute>,
        callable: Callable,
    ) -> Result<Self> {
        let lhs = primary(p, attributes, EAGER_BRACE, callable)?;
        let lookahead = ast::BinOp::from_peeker(p.peeker());
        binary(p, lhs, lookahead, 0, EAGER_BRACE)
    }

    /// ull, configurable parsing of an expression.F
    pub(crate) fn parse_with(
        p: &mut Parser<'_>,
        eager_brace: EagerBrace,
        eager_binary: EagerBinary,
        callable: Callable,
    ) -> Result<Self> {
        let mut attributes = p.parse()?;

        let expr = primary(p, &mut attributes, eager_brace, callable)?;

        let expr = if *eager_binary {
            let lookeahead = ast::BinOp::from_peeker(p.peeker());
            binary(p, expr, lookeahead, 0, eager_brace)?
        } else {
            expr
        };

        if let Some(span) = attributes.option_span() {
            return Err(compile::Error::unsupported(span, "attributes"));
        }

        Ok(expr)
    }

    /// Parse expressions that start with an identifier.
    pub(crate) fn parse_with_meta_path(
        p: &mut Parser<'_>,
        attributes: &mut Vec<ast::Attribute>,
        path: ast::Path,
        eager_brace: EagerBrace,
    ) -> Result<Self> {
        if *eager_brace && p.peek::<T!['{']>()? {
            let ident = ast::ObjectIdent::Named(path);

            return Ok(Self::Object(ast::ExprObject::parse_with_meta(
                p,
                take(attributes),
                ident,
            )?));
        }

        if p.peek::<T![!]>()? {
            return Ok(Self::MacroCall(ast::MacroCall::parse_with_meta_path(
                p,
                take(attributes),
                path,
            )?));
        }

        Ok(Self::Path(path))
    }

    pub(crate) fn peek_with_brace(p: &mut Peeker<'_>, eager_brace: EagerBrace) -> bool {
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
            K![number] => true,
            K![char] => true,
            K![byte] => true,
            K![str] => true,
            K![bytestr] => true,
            K!['label] => matches!(p.nth(1), K![:]),
            K![..] => true,
            K!['('] => true,
            K!['['] => true,
            K!['{'] if *eager_brace => true,
            _ => false,
        }
    }
}

impl Parse for Expr {
    fn parse(p: &mut Parser<'_>) -> Result<Self> {
        Self::parse_with(p, EAGER_BRACE, EAGER_BINARY, CALLABLE)
    }
}

impl Peek for Expr {
    fn peek(p: &mut Peeker<'_>) -> bool {
        Self::peek_with_brace(p, EAGER_BRACE)
    }
}

/// Primary parse entry point.
fn primary(
    p: &mut Parser<'_>,
    attributes: &mut Vec<ast::Attribute>,
    eager_brace: EagerBrace,
    callable: Callable,
) -> Result<Expr> {
    let expr = base(p, attributes, eager_brace)?;
    chain(p, expr, callable)
}

/// Parse a basic expression.
fn base(
    p: &mut Parser<'_>,
    attributes: &mut Vec<ast::Attribute>,
    eager_brace: EagerBrace,
) -> Result<Expr> {
    if let Some(path) = p.parse::<Option<ast::Path>>()? {
        return Expr::parse_with_meta_path(p, attributes, path, eager_brace);
    }

    if ast::Lit::peek_in_expr(p.peeker()) {
        return Ok(Expr::Lit(ast::ExprLit::parse_with_meta(
            p,
            take(attributes),
        )?));
    }

    let mut label = p.parse::<Option<(ast::Label, T![:])>>()?;
    let mut async_token = p.parse::<Option<T![async]>>()?;
    let mut const_token = p.parse::<Option<T![const]>>()?;
    let mut move_token = p.parse::<Option<T![move]>>()?;

    let expr = match p.nth(0)? {
        K![..] => {
            let limits = ast::ExprRangeLimits::HalfOpen(p.parse()?);
            range(p, take(attributes), None, limits, eager_brace)?
        }
        K![..=] => {
            let limits = ast::ExprRangeLimits::Closed(p.parse()?);
            range(p, take(attributes), None, limits, eager_brace)?
        }
        K![#] => {
            let ident = ast::ObjectIdent::Anonymous(p.parse()?);

            Expr::Object(ast::ExprObject::parse_with_meta(
                p,
                take(attributes),
                ident,
            )?)
        }
        K![||] | K![|] => Expr::Closure(ast::ExprClosure::parse_with_meta(
            p,
            take(attributes),
            take(&mut async_token),
            take(&mut move_token),
        )?),
        K![select] => Expr::Select(ast::ExprSelect::parse_with_attributes(p, take(attributes))?),
        K![!] | K![-] | K![&] | K![*] => Expr::Unary(ast::ExprUnary::parse_with_meta(
            p,
            take(attributes),
            eager_brace,
        )?),
        K![while] => Expr::While(ast::ExprWhile::parse_with_meta(
            p,
            take(attributes),
            take(&mut label),
        )?),
        K![loop] => Expr::Loop(ast::ExprLoop::parse_with_meta(
            p,
            take(attributes),
            take(&mut label),
        )?),
        K![for] => Expr::For(ast::ExprFor::parse_with_meta(
            p,
            take(attributes),
            take(&mut label),
        )?),
        K![let] => Expr::Let(ast::ExprLet::parse_with_meta(p, take(attributes))?),
        K![if] => Expr::If(ast::ExprIf::parse_with_meta(p, take(attributes))?),
        K![match] => Expr::Match(ast::ExprMatch::parse_with_attributes(p, take(attributes))?),
        K!['['] => Expr::Vec(ast::ExprVec::parse_with_meta(p, take(attributes))?),
        ast::Kind::Open(ast::Delimiter::Empty) => empty_group(p, take(attributes))?,
        K!['('] => paren_group(p, take(attributes))?,
        K!['{'] => Expr::Block(ast::ExprBlock::parse_with_meta(
            p,
            take(attributes),
            take(&mut async_token),
            take(&mut const_token),
            take(&mut move_token),
        )?),
        K![break] => Expr::Break(ast::ExprBreak::parse_with_meta(p, take(attributes))?),
        K![continue] => Expr::Continue(ast::ExprContinue::parse_with_meta(p, take(attributes))?),
        K![yield] => Expr::Yield(ast::ExprYield::parse_with_meta(p, take(attributes))?),
        K![return] => Expr::Return(ast::ExprReturn::parse_with_meta(p, take(attributes))?),
        _ => {
            return Err(compile::Error::expected(
                p.tok_at(0)?,
                Expectation::Expression,
            ));
        }
    };

    if let Some(span) = label.option_span() {
        return Err(compile::Error::unsupported(span, "label"));
    }

    if let Some(span) = async_token.option_span() {
        return Err(compile::Error::unsupported(span, "async modifier"));
    }

    if let Some(span) = const_token.option_span() {
        return Err(compile::Error::unsupported(span, "const modifier"));
    }

    if let Some(span) = move_token.option_span() {
        return Err(compile::Error::unsupported(span, "move modifier"));
    }

    Ok(expr)
}

/// Parse an expression chain.
fn chain(p: &mut Parser<'_>, mut expr: Expr, callable: Callable) -> Result<Expr> {
    while !p.is_eof()? {
        let is_callable = expr.is_callable(*callable);

        match p.nth(0)? {
            K!['['] if is_callable => {
                expr = Expr::Index(ast::ExprIndex {
                    attributes: expr.take_attributes(),
                    target: Box::try_new(expr)?,
                    open: p.parse()?,
                    index: p.parse()?,
                    close: p.parse()?,
                });
            }
            // Chained function call.
            K!['('] if is_callable => {
                let args = p.parse::<ast::Parenthesized<Expr, T![,]>>()?;

                expr = Expr::Call(ast::ExprCall {
                    id: Default::default(),
                    attributes: expr.take_attributes(),
                    expr: Box::try_new(expr)?,
                    args,
                });
            }
            K![?] => {
                expr = Expr::Try(ast::ExprTry {
                    attributes: expr.take_attributes(),
                    expr: Box::try_new(expr)?,
                    try_token: p.parse()?,
                });
            }
            K![=] => {
                let eq = p.parse()?;
                let rhs = Expr::parse_with(p, EAGER_BRACE, EAGER_BINARY, CALLABLE)?;

                expr = Expr::Assign(ast::ExprAssign {
                    attributes: expr.take_attributes(),
                    lhs: Box::try_new(expr)?,
                    eq,
                    rhs: Box::try_new(rhs)?,
                });
            }
            K![.] => {
                match p.nth(1)? {
                    // <expr>.await
                    K![await] => {
                        expr = Expr::Await(ast::ExprAwait {
                            attributes: expr.take_attributes(),
                            expr: Box::try_new(expr)?,
                            dot: p.parse()?,
                            await_token: p.parse()?,
                        });
                    }
                    // <expr>.field
                    K![ident] => {
                        expr = Expr::FieldAccess(ast::ExprFieldAccess {
                            attributes: expr.take_attributes(),
                            expr: Box::try_new(expr)?,
                            dot: p.parse()?,
                            expr_field: ast::ExprField::Path(p.parse()?),
                        });
                    }
                    // tuple access: <expr>.<number>
                    K![number] => {
                        expr = Expr::FieldAccess(ast::ExprFieldAccess {
                            attributes: expr.take_attributes(),
                            expr: Box::try_new(expr)?,
                            dot: p.parse()?,
                            expr_field: ast::ExprField::LitNumber(p.parse()?),
                        });
                    }
                    _ => {
                        return Err(compile::Error::new(p.span(0..1), ErrorKind::BadFieldAccess));
                    }
                }
            }
            _ => break,
        }
    }

    Ok(expr)
}

/// Parse a binary expression.
fn binary(
    p: &mut Parser<'_>,
    mut lhs: Expr,
    mut lookahead: Option<ast::BinOp>,
    min_precedence: usize,
    eager_brace: EagerBrace,
) -> Result<Expr> {
    while let Some(op) = lookahead {
        let precedence = op.precedence();

        if precedence < min_precedence {
            break;
        }

        op.advance(p)?;

        match op {
            ast::BinOp::DotDot(token) => {
                lhs = range(
                    p,
                    lhs.take_attributes(),
                    Some(Box::try_new(lhs)?),
                    ast::ExprRangeLimits::HalfOpen(token),
                    eager_brace,
                )?;
                lookahead = ast::BinOp::from_peeker(p.peeker());
                continue;
            }
            ast::BinOp::DotDotEq(token) => {
                lhs = range(
                    p,
                    lhs.take_attributes(),
                    Some(Box::try_new(lhs)?),
                    ast::ExprRangeLimits::Closed(token),
                    eager_brace,
                )?;
                lookahead = ast::BinOp::from_peeker(p.peeker());
                continue;
            }
            _ => (),
        }

        let mut rhs = primary(p, &mut Vec::new(), eager_brace, CALLABLE)?;
        lookahead = ast::BinOp::from_peeker(p.peeker());

        while let Some(next) = lookahead {
            match (precedence, next.precedence()) {
                (lh, rh) if lh < rh => {
                    // Higher precedence elements require us to recurse.
                    rhs = binary(p, rhs, Some(next), lh + 1, eager_brace)?;
                    lookahead = ast::BinOp::from_peeker(p.peeker());
                    continue;
                }
                (lh, rh) if lh == rh => {
                    if !next.is_assoc() {
                        return Err(compile::Error::new(
                            lhs.span().join(rhs.span()),
                            ErrorKind::PrecedenceGroupRequired,
                        ));
                    }
                }
                _ => {}
            };

            break;
        }

        lhs = Expr::Binary(ast::ExprBinary {
            attributes: lhs.take_attributes(),
            lhs: Box::try_new(lhs)?,
            op,
            rhs: Box::try_new(rhs)?,
        });
    }

    Ok(lhs)
}

/// Parse the tail-end of a range.
fn range(
    p: &mut Parser<'_>,
    attributes: Vec<ast::Attribute>,
    from: Option<Box<Expr>>,
    limits: ast::ExprRangeLimits,
    eager_brace: EagerBrace,
) -> Result<Expr> {
    let to = if Expr::peek_with_brace(p.peeker(), eager_brace) {
        Some(Box::try_new(Expr::parse_with(
            p,
            eager_brace,
            EAGER_BINARY,
            CALLABLE,
        )?)?)
    } else {
        None
    };

    Ok(Expr::Range(ast::ExprRange {
        attributes,
        start: from,
        limits,
        end: to,
    }))
}

/// Parsing something that opens with an empty group marker.
fn empty_group(p: &mut Parser<'_>, attributes: Vec<ast::Attribute>) -> Result<Expr> {
    let open = p.parse::<ast::OpenEmpty>()?;
    let expr = p.parse::<Expr>()?;
    let close = p.parse::<ast::CloseEmpty>()?;

    Ok(Expr::Empty(ast::ExprEmpty {
        attributes,
        open,
        expr: Box::try_new(expr)?,
        close,
    }))
}

/// Parsing something that opens with a parenthesis.
fn paren_group(p: &mut Parser<'_>, attributes: Vec<ast::Attribute>) -> Result<Expr> {
    // Empty tuple.
    if let (K!['('], K![')']) = (p.nth(0)?, p.nth(1)?) {
        return Ok(Expr::Tuple(ast::ExprTuple::parse_with_meta(p, attributes)?));
    }

    let open = p.parse::<T!['(']>()?;
    let expr = p.parse::<Expr>()?;

    // Priority expression group.
    if p.peek::<T![')']>()? {
        return Ok(Expr::Group(ast::ExprGroup {
            attributes,
            open,
            expr: Box::try_new(expr)?,
            close: p.parse()?,
        }));
    }

    // Tuple expression. These are distinguished from a group with a single item
    // by adding a `,` at the end like `(foo,)`.
    Ok(Expr::Tuple(ast::ExprTuple::parse_from_first_expr(
        p, attributes, open, expr,
    )?))
}

#[cfg(test)]
mod tests {
    use crate::ast;
    use crate::testing::rt;

    #[test]
    fn test_expr_if() {
        let expr = rt::<ast::Expr>(r#"if true {} else {}"#);
        assert!(matches!(expr, ast::Expr::If(..)));

        let expr = rt::<ast::Expr>("if 1 { } else { if 2 { } else { } }");
        assert!(matches!(expr, ast::Expr::If(..)));
    }

    #[test]
    fn test_expr_while() {
        let expr = rt::<ast::Expr>(r#"while true {}"#);
        assert!(matches!(expr, ast::Expr::While(..)));
    }

    #[test]
    fn test_expr() {
        rt::<ast::Expr>("foo[\"foo\"]");
        rt::<ast::Expr>("foo.bar()");
        rt::<ast::Expr>("var()");
        rt::<ast::Expr>("var");
        rt::<ast::Expr>("42");
        rt::<ast::Expr>("1 + 2 / 3 - 4 * 1");
        rt::<ast::Expr>("foo[\"bar\"]");
        rt::<ast::Expr>("let var = 42");
        rt::<ast::Expr>("let var = \"foo bar\"");
        rt::<ast::Expr>("var[\"foo\"] = \"bar\"");
        rt::<ast::Expr>("let var = objects[\"foo\"] + 1");
        rt::<ast::Expr>("var = 42");

        // Chained function calls.
        rt::<ast::Expr>("foo.bar.baz()");
        rt::<ast::Expr>("foo[0][1][2]");
        rt::<ast::Expr>("foo.bar()[0].baz()[1]");
        rt::<ast::Expr>("42 is i64::i64");
    }

    #[test]
    fn test_macro_call_chain() {
        rt::<ast::Expr>("format!(\"{}\", a).bar()");
    }
}
