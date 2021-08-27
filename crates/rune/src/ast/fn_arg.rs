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
    /// Function argument is a pattern binding.
    Pat(Box<ast::Pat>),
}

impl Parse for FnArg {
    fn parse(p: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(match p.nth(0)? {
            K![self] => Self::SelfValue(p.parse()?),
            _ => Self::Pat(p.parse()?),
        })
    }
}
