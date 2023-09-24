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
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, Spanned)]
#[try_clone(copy)]
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

    fn resolve(&self, cx: ResolveContext<'a>) -> Result<ast::Number> {
        fn err_span<E>(span: Span) -> impl Fn(E) -> compile::Error {
            move |_| compile::Error::new(span, ErrorKind::BadNumberLiteral)
        }

        let span = self.span;

        let text = match self.source {
            ast::NumberSource::Synthetic(id) => {
                let Some(number) = cx.storage.get_number(id) else {
                    return Err(compile::Error::new(
                        span,
                        ErrorKind::BadSyntheticId {
                            kind: SyntheticKind::Number,
                            id,
                        },
                    ));
                };

                return Ok((*number).try_clone()?);
            }
            ast::NumberSource::Text(text) => text,
        };

        let string = cx
            .sources
            .source(text.source_id, text.number)
            .ok_or_else(|| compile::Error::new(span, ErrorKind::BadSlice))?;

        let suffix = cx
            .sources
            .source(text.source_id, text.suffix)
            .ok_or_else(|| compile::Error::new(span, ErrorKind::BadSlice))?;

        let suffix = match suffix {
            "i64" => Some(ast::NumberSuffix::Int(text.suffix)),
            "f64" => Some(ast::NumberSuffix::Float(text.suffix)),
            "u8" => Some(ast::NumberSuffix::Byte(text.suffix)),
            "" => None,
            _ => {
                return Err(compile::Error::new(
                    text.suffix,
                    ErrorKind::UnsupportedSuffix,
                ))
            }
        };

        if matches!(
            (suffix, text.is_fractional),
            (Some(ast::NumberSuffix::Float(..)), _) | (None, true)
        ) {
            let number: f64 = string
                .trim_matches(|c: char| c == '_')
                .parse()
                .map_err(err_span(span))?;

            return Ok(ast::Number {
                value: ast::NumberValue::Float(number),
                suffix,
            });
        }

        let radix = match text.base {
            ast::NumberBase::Binary => 2,
            ast::NumberBase::Octal => 8,
            ast::NumberBase::Hex => 16,
            ast::NumberBase::Decimal => 10,
        };

        let number = num::BigInt::from_str_radix(string, radix).map_err(err_span(span))?;

        Ok(ast::Number {
            value: ast::NumberValue::Integer(number),
            suffix,
        })
    }
}

impl ToTokens for LitNumber {
    fn to_tokens(
        &self,
        _: &mut MacroContext<'_, '_, '_>,
        stream: &mut TokenStream,
    ) -> alloc::Result<()> {
        stream.push(ast::Token {
            span: self.span,
            kind: ast::Kind::Number(self.source),
        })
    }
}
