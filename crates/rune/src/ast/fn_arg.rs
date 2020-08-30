use crate::ast;
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::Span;

/// A single argument in a closure.
#[derive(Debug, Clone)]
pub enum FnArg {
    /// Ignoring the argument with `_`.
    Ignore(ast::Underscore),
    /// Binding the argument to an ident.
    Ident(ast::Ident),
}

impl FnArg {
    /// Get the span of the argument.
    pub fn span(&self) -> Span {
        match self {
            Self::Ignore(ignore) => ignore.span(),
            Self::Ident(ident) => ident.span(),
        }
    }
}

impl Parse for FnArg {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        Ok(match token.kind {
            ast::Kind::Underscore => Self::Ignore(parser.parse()?),
            ast::Kind::Ident => Self::Ident(parser.parse()?),
            _ => return Err(ParseError::ExpectedFunctionArgument { span: token.span }),
        })
    }
}
