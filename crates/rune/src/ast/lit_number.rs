use crate::ast::prelude::*;

use num::Num;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::LitNumber>("42");
    rt::<ast::LitNumber>("42.42");
    rt::<ast::LitNumber>("0.42");
    rt::<ast::LitNumber>("0.42e10");
}

/// A number literal.
///
/// * `42`.
/// * `4.2e10`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Spanned)]
#[non_exhaustive]
pub struct LitNumber {
    /// The span corresponding to the literal.
    pub span: Span,
    /// The source of the number.
    #[rune(skip)]
    pub source: ast::NumberSource,
}

impl Parse for LitNumber {
    fn parse(parser: &mut Parser<'_>) -> Result<Self> {
        let t = parser.next()?;

        match t.kind {
            K![number(source)] => Ok(LitNumber {
                source,
                span: t.span,
            }),
            _ => Err(compile::Error::expected(t, "number")),
        }
    }
}

impl<'a> Resolve<'a> for LitNumber {
    type Output = ast::Number;

    fn resolve(&self, ctx: ResolveContext<'a>) -> Result<ast::Number> {
        let span = self.span;

        let text = match self.source {
            ast::NumberSource::Synthetic(id) => match ctx.storage.get_number(id) {
                Some(number) => return Ok(number.clone()),
                None => {
                    return Err(compile::Error::new(
                        span,
                        ResolveErrorKind::BadSyntheticId {
                            kind: SyntheticKind::Number,
                            id,
                        },
                    ));
                }
            },
            ast::NumberSource::Text(text) => text,
        };

        let string = ctx
            .sources
            .source(text.source_id, span)
            .ok_or_else(|| compile::Error::new(span, ResolveErrorKind::BadSlice))?;

        if text.is_fractional {
            let number: f64 = string.parse().map_err(err_span(span))?;
            return Ok(ast::Number::Float(number));
        }

        let (s, radix) = match text.base {
            ast::NumberBase::Binary => (2, 2),
            ast::NumberBase::Octal => (2, 8),
            ast::NumberBase::Hex => (2, 16),
            ast::NumberBase::Decimal => (0, 10),
        };

        let number = num::BigInt::from_str_radix(&string[s..], radix).map_err(err_span(span))?;
        return Ok(ast::Number::Integer(number));

        fn err_span<E>(span: Span) -> impl Fn(E) -> compile::Error {
            move |_| compile::Error::new(span, ResolveErrorKind::BadNumberLiteral)
        }
    }
}

impl ToTokens for LitNumber {
    fn to_tokens(&self, _: &mut MacroContext<'_>, stream: &mut TokenStream) {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Number(self.source),
        });
    }
}
