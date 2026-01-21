use crate::ast::prelude::*;
use crate::compile::{num, WithSpan};

use ast::token::NumberSize;

#[test]
#[cfg(not(miri))]
fn ast_parse() {
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

impl ToAst for LitNumber {
    fn to_ast(span: Span, kind: ast::Kind) -> compile::Result<Self> {
        match kind {
            K![number(source)] => Ok(LitNumber { source, span }),
            _ => Err(compile::Error::expected(
                ast::Token { span, kind },
                Self::into_expectation(),
            )),
        }
    }

    #[inline]
    fn matches(kind: &ast::Kind) -> bool {
        matches!(kind, K![number])
    }

    #[inline]
    fn into_expectation() -> Expectation {
        Expectation::Description("number")
    }
}

impl Parse for LitNumber {
    fn parse(parser: &mut Parser<'_>) -> Result<Self> {
        let t = parser.next()?;
        Self::to_ast(t.span, t.kind)
    }
}

impl<'a> Resolve<'a> for LitNumber {
    type Output = ast::Number;

    fn resolve(&self, cx: ResolveContext<'a, '_>) -> Result<ast::Number> {
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
            "u8" => Some(ast::NumberSuffix::Unsigned(text.suffix, NumberSize::S8)),
            "u16" => Some(ast::NumberSuffix::Unsigned(text.suffix, NumberSize::S16)),
            "u32" => Some(ast::NumberSuffix::Unsigned(text.suffix, NumberSize::S32)),
            "u64" => Some(ast::NumberSuffix::Unsigned(text.suffix, NumberSize::S64)),
            "i8" => Some(ast::NumberSuffix::Signed(text.suffix, NumberSize::S8)),
            "i16" => Some(ast::NumberSuffix::Signed(text.suffix, NumberSize::S16)),
            "i32" => Some(ast::NumberSuffix::Signed(text.suffix, NumberSize::S32)),
            "i64" => Some(ast::NumberSuffix::Signed(text.suffix, NumberSize::S64)),
            "f32" | "f64" => Some(ast::NumberSuffix::Float(text.suffix)),
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
            let number: f64 = num::from_float(cx.scratch, string).with_span(span)?;

            return Ok(ast::Number {
                value: ast::NumberValue::Float(number),
                suffix,
            });
        }

        let parser = match text.base {
            ast::NumberBase::Binary => num::from_ascii_binary,
            ast::NumberBase::Octal => num::from_ascii_octal,
            ast::NumberBase::Hex => num::from_ascii_hex,
            ast::NumberBase::Decimal => num::from_ascii_decimal,
        };

        let number = parser(string.as_bytes())
            .ok_or_else(|| ErrorKind::BadNumberLiteral)
            .with_span(span)?;

        Ok(ast::Number {
            value: ast::NumberValue::Integer(number as i128),
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
