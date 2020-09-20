use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};
use runestick::Span;

/// A closure.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ExprClosure {
    /// The attributes for the async closure
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// If the closure is async or not.
    #[rune(iter)]
    pub async_: Option<ast::Async>,
    /// Arguments to the closure.
    pub args: ExprClosureArgs,
    /// The body of the closure.
    pub body: Box<ast::Expr>,
}

impl ExprClosure {
    /// Get the identifying span for this closure.
    pub fn item_span(&self) -> Span {
        if let Some(async_) = &self.async_ {
            async_.span().join(self.args.span())
        } else {
            self.args.span()
        }
    }

    /// Parse the closure attaching the given attributes
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        let async_ = parser.parse()?;

        let args = if let Some(token) = parser.parse::<Option<ast::Or>>()? {
            ExprClosureArgs::Empty { token }
        } else {
            let open = parser.parse()?;
            let mut args = Vec::new();

            while !parser.peek::<ast::Pipe>()? {
                let arg = parser.parse()?;

                let comma = parser.parse::<Option<ast::Comma>>()?;
                let is_end = comma.is_none();
                args.push((arg, comma));

                if is_end {
                    break;
                }
            }

            let close = parser.parse()?;

            ExprClosureArgs::List { open, args, close }
        };

        Ok(Self {
            attributes,
            async_,
            args,
            body: Box::new(parser.parse()?),
        })
    }
}

/// Parse implementation for a function.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ExprClosure>("async || 42").unwrap();
/// parse_all::<ast::ExprClosure>("|| 42").unwrap();
/// parse_all::<ast::ExprClosure>("|| { 42 }").unwrap();
///
/// let expr = parse_all::<ast::ExprClosure>("#[retry(n=3)]  || 43").unwrap();
/// assert_eq!(expr.attributes.len(), 1);
///
/// let expr = parse_all::<ast::ExprClosure>("#[retry(n=3)] async || 43").unwrap();
/// assert_eq!(expr.attributes.len(), 1);
/// ```
impl Parse for ExprClosure {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        Self::parse_with_attributes(parser, attributes)
    }
}

#[derive(Debug, Clone, ToTokens)]
pub enum ExprClosureArgs {
    Empty {
        /// The `||` token.
        token: ast::Or,
    },
    List {
        /// The opening pipe for the argument group.
        open: ast::Pipe,
        /// The arguments of the function.
        args: Vec<(ast::FnArg, Option<ast::Comma>)>,
        /// The closening pipe for the argument group.
        close: ast::Pipe,
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

    /// Iterate over all arguments.
    pub fn as_slice(&self) -> &[(ast::FnArg, Option<ast::Comma>)] {
        match self {
            Self::Empty { .. } => &[],
            Self::List { args, .. } => &args[..],
        }
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
