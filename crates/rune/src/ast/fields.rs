use core::iter;
use core::slice;

use crate::ast::prelude::*;

/// An item body declaration.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, OptionSpanned)]
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

type ToField = fn(&(ast::Field, Option<T![,]>)) -> &ast::Field;

fn to_field((field, _): &(ast::Field, Option<T![,]>)) -> &ast::Field {
    field
}

impl<'a> IntoIterator for &'a Fields {
    type Item = &'a ast::Field;
    type IntoIter = iter::Map<slice::Iter<'a, (ast::Field, Option<T![,]>)>, ToField>;

    fn into_iter(self) -> Self::IntoIter {
        static STATIC: &[(ast::Field, Option<T![,]>); 0] = &[];

        match self {
            Fields::Named(fields) => fields.iter().map(to_field as ToField),
            Fields::Unnamed(fields) => fields.iter().map(to_field as ToField),
            Fields::Empty => STATIC.iter().map(to_field as ToField),
        }
    }
}
