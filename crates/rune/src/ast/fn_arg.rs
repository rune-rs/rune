use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// A single argument in a closure.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::FnArg>("self");
/// testing::roundtrip::<ast::FnArg>("_");
/// testing::roundtrip::<ast::FnArg>("abc");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum FnArg {
    /// The `self` parameter.
    SelfValue(T![self]),
    /// Ignoring the argument with `_`.
    Ignore(T![_]),
    /// Binding the argument to an ident.
    Ident(ast::Ident),
}

impl Parse for FnArg {
    fn parse(p: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(match p.nth(0)? {
            K![self] => Self::SelfValue(p.parse()?),
            K![_] => Self::Ignore(p.parse()?),
            K![ident(..)] => Self::Ident(p.parse()?),
            _ => {
                return Err(ParseError::expected(
                    p.token(0)?,
                    "expected function argument",
                ))
            }
        })
    }
}
