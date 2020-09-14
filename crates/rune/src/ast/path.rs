use crate::ast;
use crate::ast::{Kind, Token};
use crate::error::ParseError;
use crate::parser::Parser;
use crate::{IntoTokens, Parse, Peek, Resolve, Storage};
use runestick::{Source, Span};
use std::borrow::Cow;

/// A path, where each element is separated by a `::`.
#[derive(Debug, Clone)]
pub struct Path {
    /// The first component in the path.
    pub first: ast::Ident,
    /// The rest of the components in the path.
    pub rest: Vec<(ast::Scope, ast::Ident)>,
    /// Trailing scope.
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

    /// Calculate the full span of the path.
    pub fn span(&self) -> Span {
        if let Some(trailing) = &self.trailing {
            return self.first.span().join(trailing.span());
        }

        if let Some((_, ident)) = self.rest.last() {
            return self.first.span().join(ident.span());
        }

        self.first.span()
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
    fn peek(t1: Option<Token>, _: Option<Token>) -> bool {
        let t1 = match t1 {
            Some(t1) => t1,
            None => return false,
        };

        matches!(t1.kind, Kind::Ident(..))
    }
}

impl Parse for Path {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(Self {
            first: parser.parse()?,
            rest: parser.parse()?,
            trailing: parser.parse()?,
        })
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

impl IntoTokens for Path {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        self.first.into_tokens(context, stream);

        for (sep, rest) in &self.rest {
            sep.into_tokens(context, stream);
            rest.into_tokens(context, stream);
        }
    }
}
