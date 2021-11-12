use crate::ast::prelude::*;

/// A closure expression.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprClosure>("async || 42");
/// testing::roundtrip::<ast::ExprClosure>("|| 42");
/// testing::roundtrip::<ast::ExprClosure>("|| { 42 }");
/// testing::roundtrip::<ast::ExprClosure>("move || { 42 }");
/// testing::roundtrip::<ast::ExprClosure>("async move || { 42 }");
///
/// let expr = testing::roundtrip::<ast::ExprClosure>("#[retry(n=3)]  || 43");
/// assert_eq!(expr.attributes.len(), 1);
///
/// let expr = testing::roundtrip::<ast::ExprClosure>("#[retry(n=3)] async || 43");
/// assert_eq!(expr.attributes.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
pub struct ExprClosure {
    /// Opaque identifier for the closure.
    #[rune(id)]
    pub id: Option<Id>,
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
    /// The body of the closure.
    pub body: ast::Expr,
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

impl Opaque for ExprClosure {
    fn id(&self) -> Option<Id> {
        self.id
    }
}

expr_parse!(Closure, ExprClosure, "closure expression");

#[derive(Debug, Clone, PartialEq, Eq, ToTokens)]
pub enum ExprClosureArgs {
    Empty {
        /// The `||` token.
        token: T![||],
    },
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
    /// The number of arguments the closure takes.
    pub fn len(&self) -> usize {
        match self {
            Self::Empty { .. } => 0,
            Self::List { args, .. } => args.len(),
        }
    }

    /// Get a slice over all arguments.
    pub fn as_slice(&self) -> &[(ast::FnArg, Option<T![,]>)] {
        match self {
            Self::Empty { .. } => &[],
            Self::List { args, .. } => &args[..],
        }
    }

    /// Get a mutable slice over all arguments.
    pub fn as_slice_mut(&mut self) -> &mut [(ast::FnArg, Option<T![,]>)] {
        match self {
            Self::Empty { .. } => &mut [],
            Self::List { args, .. } => &mut args[..],
        }
    }
}

impl Parse for ExprClosureArgs {
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        if let Some(token) = p.parse::<Option<T![||]>>()? {
            return Ok(ExprClosureArgs::Empty { token });
        }

        let open = p.parse()?;
        let mut args = Vec::new();

        while !p.peek::<T![|]>()? {
            let arg = p.parse()?;

            let comma = p.parse::<Option<T![,]>>()?;
            let is_end = comma.is_none();
            args.push((arg, comma));

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
