use crate::ast;
use crate::{
    Parse, ParseError, ParseErrorKind, Parser, Peek, Resolve, ResolveOwned, Spanned, Storage,
    ToTokens,
};
use runestick::Source;
use std::borrow::Cow;

/// An identifier, like `foo` or `Hello`.".
#[derive(Debug, Clone, Copy, PartialEq, Eq, ToTokens, Spanned)]
pub struct Ident {
    /// The kind of the identifier.
    pub token: ast::Token,
    /// The kind of the identifier.
    #[rune(skip)]
    pub source: ast::StringSource,
}

impl Ident {
    /// Construct a new synthetic identifier.
    ///
    /// # Panics
    ///
    /// This will panic if it's called outside of a macro context.
    pub fn new(ident: &str) -> Self {
        crate::macros::current_context(|ctx| ctx.ident(ident))
    }
}

impl Parse for Ident {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        match token.kind {
            ast::Kind::Ident(source) => Ok(Self { token, source }),
            _ => Err(ParseError::new(
                token,
                ParseErrorKind::TokenMismatch {
                    expected: ast::Kind::Ident(ast::StringSource::Text),
                    actual: token.kind,
                },
            )),
        }
    }
}

impl Peek for Ident {
    fn peek(p1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        match p1 {
            Some(p1) => matches!(p1.kind, ast::Kind::Ident(..)),
            _ => false,
        }
    }
}

impl<'a> Resolve<'a> for Ident {
    type Output = Cow<'a, str>;

    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<Cow<'a, str>, ParseError> {
        let span = self.token.span();

        match self.source {
            ast::StringSource::Text => {
                let ident = source
                    .source(span)
                    .ok_or_else(|| ParseError::new(span, ParseErrorKind::BadSlice))?;

                Ok(Cow::Borrowed(ident))
            }
            ast::StringSource::Synthetic(id) => {
                let ident = storage.get_string(id).ok_or_else(|| {
                    ParseError::new(span, ParseErrorKind::BadSyntheticId { kind: "label", id })
                })?;

                Ok(Cow::Owned(ident))
            }
        }
    }
}

impl ResolveOwned for Ident {
    type Owned = String;

    fn resolve_owned(&self, storage: &Storage, source: &Source) -> Result<Self::Owned, ParseError> {
        let output = self.resolve(storage, source)?;

        match output {
            Cow::Borrowed(borrowed) => Ok(borrowed.to_owned()),
            Cow::Owned(owned) => Ok(owned),
        }
    }
}
