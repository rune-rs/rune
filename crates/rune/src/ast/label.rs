use crate::ast::prelude::*;

/// A label, like `'foo`.
///
/// Custom labels are constructed in macros using
/// [MacroContext::label][crate::macros::MacroContext::label].
///
/// # Examples
///
/// Constructing a label:
///
/// ```
/// use rune::ast;
/// use rune::macros::MacroContext;
///
/// MacroContext::test(|ctx| {
///     let lit = ctx.label("foo");
///     assert!(matches!(lit, ast::Label { .. }))
/// });
/// ```
///
/// Example labels:
///
/// ```
/// use rune::{ast, testing};
///
/// testing::roundtrip::<ast::Label>("'foo");
/// testing::roundtrip::<ast::Label>("'barify42");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Spanned)]
#[non_exhaustive]
pub struct Label {
    /// The token of the label.
    pub span: Span,
    /// The source of the label.
    #[rune(skip)]
    pub source: ast::LitSource,
}

impl Parse for Label {
    fn parse(p: &mut Parser<'_>) -> Result<Self, ParseError> {
        let t = p.next()?;

        match t.kind {
            K!['label(source)] => Ok(Self {
                span: t.span,
                source,
            }),
            _ => Err(ParseError::expected(t, "label")),
        }
    }
}

impl Peek for Label {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!(p.nth(0), K!['label])
    }
}

impl<'a> Resolve<'a> for Label {
    type Output = &'a str;

    fn resolve(&self, ctx: ResolveContext<'a>) -> Result<&'a str, ResolveError> {
        let span = self.span;

        match self.source {
            ast::LitSource::Text(source_id) => {
                let ident = ctx
                    .sources
                    .source(source_id, span.trim_start(1u32))
                    .ok_or_else(|| ResolveError::new(span, ResolveErrorKind::BadSlice))?;

                Ok(ident)
            }
            ast::LitSource::Synthetic(id) => {
                let ident = ctx.storage.get_string(id).ok_or_else(|| {
                    ResolveError::new(
                        span,
                        ResolveErrorKind::BadSyntheticId {
                            kind: SyntheticKind::Ident,
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

impl ToTokens for Label {
    fn to_tokens(&self, _: &mut MacroContext<'_>, stream: &mut TokenStream) {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Label(self.source),
        });
    }
}
