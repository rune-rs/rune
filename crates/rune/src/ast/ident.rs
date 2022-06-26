use crate::ast::prelude::*;

/// An identifier, like `foo` or `Hello`.
///
/// Custom identifiers are constructed in macros using
/// [MacroContext::ident][crate::macros::MacroContext::ident].
///
/// # Examples
///
/// Constructing an identifier:
///
/// ```
/// use rune::ast;
/// use rune::macros::MacroContext;
///
/// MacroContext::test(|ctx| {
///     let lit = ctx.ident("foo");
///     assert!(matches!(lit, ast::Ident { .. }))
/// });
/// ```
///
/// Example identifiers:
///
/// ```
/// use rune::{ast, testing};
///
/// testing::roundtrip::<ast::Ident>("foo");
/// testing::roundtrip::<ast::Ident>("a42");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Spanned)]
#[non_exhaustive]
pub struct Ident {
    /// The span of the identifier.
    pub span: Span,
    /// The kind of the identifier.
    #[rune(skip)]
    pub source: ast::LitSource,
}

impl Parse for Ident {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let t = parser.next()?;

        match t.kind {
            ast::Kind::Ident(source) => Ok(Self {
                span: t.span,
                source,
            }),
            _ => Err(ParseError::expected(t, "ident")),
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

    fn resolve(&self, ctx: ResolveContext<'a>) -> Result<&'a str, ResolveError> {
        let span = self.span;

        match self.source {
            ast::LitSource::Text(source_id) => {
                let ident = ctx
                    .sources
                    .source(source_id, span)
                    .ok_or_else(|| ResolveError::new(span, ResolveErrorKind::BadSlice))?;

                Ok(ident)
            }
            ast::LitSource::Synthetic(id) => {
                let ident = ctx.storage.get_string(id).ok_or_else(|| {
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

impl ToTokens for Ident {
    fn to_tokens(&self, _: &mut MacroContext<'_>, stream: &mut TokenStream) {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Ident(self.source),
        });
    }
}
