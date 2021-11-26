use crate::ast::prelude::*;

/// An identifier, like `foo` or `Hello`.".
#[derive(Debug, Clone, Copy, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct Ident {
    /// The kind of the identifier.
    pub token: ast::Token,
    /// The kind of the identifier.
    #[rune(skip)]
    pub source: ast::LitSource,
}

impl Ident {
    /// Construct a new identifier from the given string from inside of a macro
    /// context.
    ///
    /// This constructor must only be used inside of a macro.
    pub fn new(ctx: &mut MacroContext<'_>, ident: &str) -> Self {
        Self::new_with(ident, ctx.macro_span(), &mut ctx.q_mut().storage)
    }

    /// Construct a new identifier from the given string.
    ///
    /// This does not panic when called outside of a macro but requires access
    /// to a `span` and `storage`.
    pub(crate) fn new_with(ident: &str, span: Span, storage: &mut Storage) -> ast::Ident {
        let id = storage.insert_str(ident);
        let source = ast::LitSource::Synthetic(id);

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
            _ => Err(ParseError::expected(token, "ident")),
        }
    }
}

impl Peek for Ident {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!(p.nth(0), K![ident])
    }
}

impl<'a> Resolve<'a> for Ident {
    type Output = &'a str;

    fn resolve(&self, storage: &'a Storage, sources: &'a Sources) -> Result<&'a str, ResolveError> {
        let span = self.token.span();

        match self.source {
            ast::LitSource::Text(source_id) => {
                let ident = sources
                    .source(source_id, span)
                    .ok_or_else(|| ResolveError::new(span, ResolveErrorKind::BadSlice))?;

                Ok(ident)
            }
            ast::LitSource::Synthetic(id) => {
                let ident = storage.get_string(id).ok_or_else(|| {
                    ResolveError::new(
                        span,
                        ResolveErrorKind::BadSyntheticId {
                            kind: SyntheticKind::Label,
                            id,
                        },
                    )
                })?;

                Ok(ident)
            }
            ast::LitSource::BuiltIn(builtin) => Ok(builtin.as_str()),
        }
    }
}
