use crate::ast;
use crate::{Ast, Parse, ParseError, ParseErrorKind, Parser};

/// A single argument in a closure.
#[derive(Debug, Clone, Ast)]
pub enum FnArg {
    /// The `self` parameter.
    Self_(ast::Self_),
    /// Ignoring the argument with `_`.
    Ignore(ast::Underscore),
    /// Binding the argument to an ident.
    Ident(ast::Ident),
}

impl Parse for FnArg {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        Ok(match token.kind {
            ast::Kind::Self_ => Self::Self_(parser.parse()?),
            ast::Kind::Underscore => Self::Ignore(parser.parse()?),
            ast::Kind::Ident(..) => Self::Ident(parser.parse()?),
            _ => {
                return Err(ParseError::new(
                    token,
                    ParseErrorKind::ExpectedFunctionArgument,
                ))
            }
        })
    }
}
