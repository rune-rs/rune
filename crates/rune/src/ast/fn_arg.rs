use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::FnArg>("self");
    rt::<ast::FnArg>("_");
    rt::<ast::FnArg>("abc");
}

/// A single argument in a closure.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum FnArg {
    /// The `self` parameter.
    SelfValue(T![self]),
    /// Function argument is a pattern binding.
    Pat(ast::Pat),
}

impl Parse for FnArg {
    fn parse(p: &mut Parser<'_>) -> Result<Self> {
        Ok(match p.nth(0)? {
            K![self] => Self::SelfValue(p.parse()?),
            _ => Self::Pat(p.parse()?),
        })
    }
}
