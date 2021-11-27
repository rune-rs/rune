use crate::ast::prelude::*;

/// A single argument in a closure.
///
/// # Examples
///
/// ```
/// use rune::{ast, testing};
///
/// testing::roundtrip::<ast::FnArg>("self");
/// testing::roundtrip::<ast::FnArg>("_");
/// testing::roundtrip::<ast::FnArg>("abc");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum FnArg {
    /// The `self` parameter.
    SelfValue(T![self]),
    /// Function argument is a pattern binding.
    Pat(ast::Pat),
}

impl Parse for FnArg {
    fn parse(p: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(match p.nth(0)? {
            K![self] => Self::SelfValue(p.parse()?),
            _ => Self::Pat(p.parse()?),
        })
    }
}
