use crate::ast::prelude::*;

/// An item body declaration.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, OptionSpanned)]
#[non_exhaustive]
pub enum Fields {
    /// A regular body.
    Named(ast::Braced<ast::Field, T![,]>),
    /// A tuple body.
    Unnamed(ast::Parenthesized<ast::Field, T![,]>),
    /// An empty body.
    Empty,
}

impl Fields {
    /// If the body needs to be terminated with a semicolon.
    pub(crate) fn needs_semi_colon(&self) -> bool {
        matches!(self, Self::Empty | Self::Unnamed(..))
    }

    /// Iterate over the fields of the body.
    pub(crate) fn fields(&self) -> impl Iterator<Item = &'_ (ast::Field, Option<T![,]>)> {
        match self {
            Fields::Empty => IntoIterator::into_iter(&[]),
            Fields::Unnamed(body) => body.iter(),
            Fields::Named(body) => body.iter(),
        }
    }
}

impl Parse for Fields {
    fn parse(p: &mut Parser<'_>) -> Result<Self> {
        Ok(match p.nth(0)? {
            K!['('] => Self::Unnamed(p.parse()?),
            K!['{'] => Self::Named(p.parse()?),
            _ => Self::Empty,
        })
    }
}
