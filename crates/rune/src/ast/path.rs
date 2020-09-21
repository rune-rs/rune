use crate::ast;
use crate::{Parse, ParseError, ParseErrorKind, Parser, Peek, Resolve, Spanned, Storage, ToTokens};
use runestick::Source;
use std::borrow::Cow;

/// A path, where each element is separated by a `::`.
#[derive(Debug, Clone, Parse, ToTokens, Spanned)]
pub struct Path {
    /// The optional leading colon `::`
    #[rune(iter)]
    pub leading_colon: Option<ast::Scope>,
    /// The first component in the path.
    pub first: PathSegment,
    /// The rest of the components in the path.
    #[rune(iter)]
    pub rest: Vec<(ast::Scope, PathSegment)>,
    /// Trailing scope.
    #[rune(iter)]
    pub trailing: Option<ast::Scope>,
}

impl Path {
    /// Borrow as an identifier used for field access calls.
    ///
    /// This is only allowed if there are no other path components
    /// and the PathSegment is not `Crate` or `Super`.
    pub fn try_as_ident(&self) -> Option<&ast::Ident> {
        if self.rest.is_empty() && self.trailing.is_none() {
            self.first.try_as_ident()
        } else {
            None
        }
    }

    /// Iterate over all components in path.
    pub fn into_components(&self) -> impl Iterator<Item = &'_ PathSegment> + '_ {
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
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        matches!(peek!(t1).kind, ast::Kind::ColonColon) || PathSegment::peek(t1, t2)
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

#[derive(Debug, Clone, ToTokens, Spanned)]
pub enum PathSegment {
    Ident(ast::Ident),
    Crate(ast::Crate),
    Super(ast::Super),
}

impl PathSegment {
    /// Borrow as an identifier.
    ///
    /// This is only allowed if the PathSegment is `Ident(_)`
    /// and not `Crate` or `Super`.
    pub fn try_as_ident(&self) -> Option<&ast::Ident> {
        if let PathSegment::Ident(ident) = self {
            Some(ident)
        } else {
            None
        }
    }
}

impl Parse for PathSegment {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;
        match token.kind {
            ast::Kind::Ident(_) => Ok(PathSegment::Ident(parser.parse()?)),
            ast::Kind::Crate => Ok(PathSegment::Crate(parser.parse()?)),
            ast::Kind::Super => Ok(PathSegment::Super(parser.parse()?)),
            _ => {
                return Err(ParseError::new(
                    token,
                    ParseErrorKind::TokenMismatch {
                        expected: ast::Kind::Ident(ast::StringSource::Text),
                        actual: token.kind,
                    },
                ))
            }
        }
    }
}

impl Peek for PathSegment {
    fn peek(t1: Option<ast::Token>, _t2: Option<ast::Token>) -> bool {
        matches!(peek!(t1).kind, ast::Kind::Ident(_) | ast::Kind::Crate | ast::Kind::Super)
    }
}

impl<'a> Resolve<'a> for PathSegment {
    type Output = Cow<'a, str>;

    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<Cow<'a, str>, ParseError> {
        let ident_span = match self {
            Self::Ident(ident) => return ident.resolve(storage, source),
            Self::Super(super_) => super_.span(),
            Self::Crate(crate_) => crate_.span(),
        };

        let ident = source
            .source(ident_span)
            .ok_or_else(|| ParseError::new(ident_span, ParseErrorKind::BadSlice))?;

        Ok(Cow::Borrowed(ident))
    }
}
