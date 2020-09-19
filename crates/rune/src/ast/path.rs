use crate::ast;
use crate::{Ast, Parse, ParseError, Peek, Resolve, Spanned, Storage};
use runestick::Source;
use std::borrow::Cow;

/// A path, where each element is separated by a `::`.
#[derive(Debug, Clone, Ast, Spanned, Parse)]
pub struct Path {
    /// The first component in the path.
    pub first: ast::Ident,
    /// The rest of the components in the path.
    #[spanned(last)]
    pub rest: Vec<(ast::Scope, ast::Ident)>,
    /// Trailing scope.
    #[spanned(last)]
    pub trailing: Option<ast::Scope>,
}

impl Path {
    /// Borrow as an identifier used for field access calls.
    ///
    /// This is only allowed if there are no other path components.
    pub fn try_as_ident(&self) -> Option<&ast::Ident> {
        if self.rest.is_empty() && self.trailing.is_none() {
            Some(&self.first)
        } else {
            None
        }
    }

    /// Iterate over all components in path.
    pub fn into_components(&self) -> impl Iterator<Item = &'_ ast::Ident> + '_ {
        let mut first = Some(&self.first);
        let mut it = self.rest.iter();

        std::iter::from_fn(move || {
            if let Some(first) = first.take() {
                return Some(first);
            }

            Some(&it.next()?.1)
        })
    }
}

impl Peek for Path {
    fn peek(t1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        matches!(peek!(t1).kind, ast::Kind::Ident(..))
    }
}

impl<'a> Resolve<'a> for Path {
    type Output = Vec<Cow<'a, str>>;

    fn resolve(
        &self,
        storage: &Storage,
        source: &'a Source,
    ) -> Result<Vec<Cow<'a, str>>, ParseError> {
        let mut output = Vec::new();

        output.push(self.first.resolve(storage, source)?);

        for (_, ident) in &self.rest {
            output.push(ident.resolve(storage, source)?);
        }

        Ok(output)
    }
}
