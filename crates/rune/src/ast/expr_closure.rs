use crate::ast::prelude::*;

#[test]
#[cfg(not(miri))]
fn ast_parse() {
    rt::<ast::ExprClosure>("async || 42");
    rt::<ast::ExprClosure>("|| 42");
    rt::<ast::ExprClosure>("|| { 42 }");
    rt::<ast::ExprClosure>("move || { 42 }");
    rt::<ast::ExprClosure>("async move || { 42 }");

    let expr = rt::<ast::ExprClosure>("#[retry(n=3)]  || 43");
    assert_eq!(expr.attributes.len(), 1);

    let expr = rt::<ast::ExprClosure>("#[retry(n=3)] async || 43");
    assert_eq!(expr.attributes.len(), 1);
}

#[test]
#[cfg(not(miri))]
fn ast_parse_with_return_type() {
    // Closure with return type annotation
    rt::<ast::ExprClosure>("|| -> i64 { 42 }");
    rt::<ast::ExprClosure>("|x: i64| -> i64 { x + 1 }");
    rt::<ast::ExprClosure>("|a: i64, b: i64| -> i64 { a + b }");
    rt::<ast::ExprClosure>("async |url: String| -> String { url }");
    rt::<ast::ExprClosure>("move || -> () { }");

    // Verify return type is captured
    let expr = rt::<ast::ExprClosure>("|x: i64| -> String { x.to_string() }");
    assert!(expr.return_type.is_some());

    // Without return type
    let expr = rt::<ast::ExprClosure>("|x: i64| { x + 1 }");
    assert!(expr.return_type.is_none());
}

/// A closure expression.
///
/// * `|| <expr>`.
/// * `async || <expr>`.
/// * `|x: i64| -> i64 { x + 1 }` (with return type annotation).
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
#[non_exhaustive]
pub struct ExprClosure {
    /// The attributes for the async closure
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// If the closure is async or not.
    #[rune(iter, meta)]
    pub async_token: Option<T![async]>,
    /// If the closure moves data into it.
    #[rune(iter, meta)]
    pub move_token: Option<T![move]>,
    /// Arguments to the closure.
    pub args: ExprClosureArgs,
    /// Optional return type annotation (gradual typing).
    #[rune(iter)]
    pub return_type: Option<(T![->], Box<ast::Type>)>,
    /// The body of the closure.
    pub body: Box<ast::Expr>,
    /// Opaque identifier for the closure.
    #[rune(skip)]
    pub(crate) id: ItemId,
}

impl ExprClosure {
    /// Get the identifying span for this closure.
    pub fn item_span(&self) -> Span {
        if let Some(async_) = &self.async_token {
            async_.span().join(self.args.span())
        } else {
            self.args.span()
        }
    }
}

expr_parse!(Closure, ExprClosure, "closure expression");

/// Representation of closure arguments.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens)]
#[non_exhaustive]
pub enum ExprClosureArgs {
    /// Closure has no arguments.
    Empty {
        /// The `||` token.
        token: T![||],
    },
    /// Closure has a list of arguments.
    List {
        /// The opening pipe for the argument group.
        open: T![|],
        /// The arguments of the function.
        args: Vec<(ast::FnArg, Option<T![,]>)>,
        /// The closening pipe for the argument group.
        close: T![|],
    },
}

impl ExprClosureArgs {
    /// Get a slice over all arguments.
    pub(crate) fn as_slice(&self) -> &[(ast::FnArg, Option<T![,]>)] {
        match self {
            Self::Empty { .. } => &[],
            Self::List { args, .. } => &args[..],
        }
    }

    /// Get a mutable slice over all arguments.
    pub(crate) fn as_slice_mut(&mut self) -> &mut [(ast::FnArg, Option<T![,]>)] {
        match self {
            Self::Empty { .. } => &mut [],
            Self::List { args, .. } => &mut args[..],
        }
    }
}

impl Parse for ExprClosureArgs {
    fn parse(p: &mut Parser<'_>) -> Result<Self> {
        if let Some(token) = p.parse::<Option<T![||]>>()? {
            return Ok(ExprClosureArgs::Empty { token });
        }

        let open = p.parse()?;
        let mut args = Vec::new();

        while !p.peek::<T![|]>()? {
            let arg = p.parse()?;

            let comma = p.parse::<Option<T![,]>>()?;
            let is_end = comma.is_none();
            args.try_push((arg, comma))?;

            if is_end {
                break;
            }
        }

        Ok(ExprClosureArgs::List {
            open,
            args,
            close: p.parse()?,
        })
    }
}

impl Spanned for ExprClosureArgs {
    fn span(&self) -> Span {
        match self {
            Self::Empty { token } => token.span(),
            Self::List { open, close, .. } => open.span().join(close.span()),
        }
    }
}
