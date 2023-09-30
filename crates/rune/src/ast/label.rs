use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::Label>("'foo");
    rt::<ast::Label>("'barify42");
}

/// A label, like `'foo`.
///
/// Custom labels are constructed in macros using
/// [MacroContext::label][crate::macros::MacroContext::label].
///
/// ```
/// use rune::ast;
/// use rune::macros;
///
/// macros::test(|cx| {
///     let lit = cx.label("foo")?;
///     assert!(matches!(lit, ast::Label { .. }));
///     Ok(())
/// })?;
/// # Ok::<_, rune::support::Error>(())
/// ```
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, Spanned)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Label {
    /// The token of the label.
    pub span: Span,
    /// The source of the label.
    #[rune(skip)]
    pub source: ast::LitSource,
}

impl Parse for Label {
    fn parse(p: &mut Parser<'_>) -> Result<Self> {
        let t = p.next()?;

        match t.kind {
            K!['label(source)] => Ok(Self {
                span: t.span,
                source,
            }),
            _ => Err(compile::Error::expected(t, "label")),
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

    fn resolve(&self, cx: ResolveContext<'a>) -> Result<&'a str> {
        let span = self.span;

        match self.source {
            ast::LitSource::Text(source_id) => {
                let ident = cx
                    .sources
                    .source(source_id, span.trim_start(1u32))
                    .ok_or_else(|| compile::Error::new(span, ErrorKind::BadSlice))?;

                Ok(ident)
            }
            ast::LitSource::Synthetic(id) => {
                let ident = cx.storage.get_string(id).ok_or_else(|| {
                    compile::Error::new(
                        span,
                        ErrorKind::BadSyntheticId {
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
    fn to_tokens(
        &self,
        _: &mut MacroContext<'_, '_, '_>,
        stream: &mut TokenStream,
    ) -> alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Label(self.source),
        })
    }
}
