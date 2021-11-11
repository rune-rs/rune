use crate::ast;
use crate::{
    MacroContext, Parse, ParseError, Parser, Peek, Peeker, Resolve, ResolveError, ResolveErrorKind,
    ResolveOwned, Sources, Spanned, Storage, ToTokens,
};
use runestick::Span;
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
    /// Construct a new identifier from the given string from inside of a macro
    /// context.
    ///
    /// This constructor must only be used inside of a macro.
    pub fn new(ctx: &mut MacroContext<'_>, ident: &str) -> Self {
        Self::new_with(ident, ctx.macro_span(), ctx.storage_mut())
    }

    /// Construct a new identifier from the given string.
    ///
    /// This does not panic when called outside of a macro but requires access
    /// to a `span` and `storage`.
    pub(crate) fn new_with(ident: &str, span: Span, storage: &mut Storage) -> ast::Ident {
        let id = storage.insert_str(ident);
        let source = ast::StringSource::Synthetic(id);

        ast::Ident {
            token: ast::Token {
                span,
                kind: ast::Kind::Ident(source),
            },
            source,
        }
    }
}

impl Parse for Ident {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.next()?;

        match token.kind {
            ast::Kind::Ident(source) => Ok(Self { token, source }),
            _ => Err(ParseError::expected(&token, "ident")),
        }
    }
}

impl Peek for Ident {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!(p.nth(0), K![ident])
    }
}

impl<'a> Resolve<'a> for Ident {
    type Output = Cow<'a, str>;

    fn resolve(
        &self,
        storage: &Storage,
        sources: &'a Sources,
    ) -> Result<Cow<'a, str>, ResolveError> {
        let span = self.token.span();

        match self.source {
            ast::StringSource::Text(source_id) => {
                let ident = sources
                    .source(source_id, span)
                    .ok_or_else(|| ResolveError::new(span, ResolveErrorKind::BadSlice))?;

                Ok(Cow::Borrowed(ident))
            }
            ast::StringSource::Synthetic(id) => {
                let ident = storage.get_string(id).ok_or_else(|| {
                    ResolveError::new(span, ResolveErrorKind::BadSyntheticId { kind: "label", id })
                })?;

                Ok(Cow::Owned(ident.clone()))
            }
            ast::StringSource::BuiltIn(builtin) => Ok(Cow::Borrowed(builtin.as_str())),
        }
    }
}

impl ResolveOwned for Ident {
    type Owned = String;

    fn resolve_owned(
        &self,
        storage: &Storage,
        sources: &Sources,
    ) -> Result<Self::Owned, ResolveError> {
        let output = self.resolve(storage, sources)?;

        match output {
            Cow::Borrowed(borrowed) => Ok(borrowed.to_owned()),
            Cow::Owned(owned) => Ok(owned),
        }
    }
}
