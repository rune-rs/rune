use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::Ident>("foo");
    rt::<ast::Ident>("a42");
    rt::<ast::Ident>("_ignored");
}

/// An identifier, like `foo` or `Hello`.
///
/// # Constructing identifiers
///
/// Inside of a macro context, identifiers have to be constructed through
/// [MacroContext::ident][crate::macros::MacroContext::ident].
///
/// ```
/// use rune::ast;
/// use rune::macros;
///
/// macros::test(|cx| {
///     let lit = cx.ident("foo")?;
///     assert!(matches!(lit, ast::Ident { .. }));
///     Ok(())
/// })?;
/// # Ok::<_, rune::support::Error>(())
/// ```
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, Spanned)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Ident {
    /// The span of the identifier.
    pub span: Span,
    /// The kind of the identifier.
    #[rune(skip)]
    pub source: ast::LitSource,
}

impl Parse for Ident {
    fn parse(parser: &mut Parser<'_>) -> Result<Self> {
        let t = parser.next()?;

        match t.kind {
            ast::Kind::Ident(source) => Ok(Self {
                span: t.span,
                source,
            }),
            _ => Err(compile::Error::expected(t, "ident")),
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

    fn resolve(&self, cx: ResolveContext<'a>) -> Result<&'a str> {
        let span = self.span;

        match self.source {
            ast::LitSource::Text(source_id) => {
                let ident = cx
                    .sources
                    .source(source_id, span)
                    .ok_or_else(|| compile::Error::new(span, ErrorKind::BadSlice))?;

                Ok(ident)
            }
            ast::LitSource::Synthetic(id) => {
                let ident = cx.storage.get_string(id).ok_or_else(|| {
                    compile::Error::new(
                        span,
                        ErrorKind::BadSyntheticId {
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
    fn to_tokens(
        &self,
        _: &mut MacroContext<'_, '_, '_>,
        stream: &mut TokenStream,
    ) -> alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Ident(self.source),
        })
    }
}
