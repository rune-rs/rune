use crate::ast;
use crate::{Parse, ParseError, ParseErrorKind, Parser, Resolve, Spanned, Storage, ToTokens};
use runestick::{Source, Span};

/// A number literal.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct LitNumber {
    /// The token corresponding to the literal.
    token: ast::Token,
    /// The source of the number.
    #[rune(skip)]
    source: ast::NumberSource,
}

/// Parse a number literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::LitNumber>("42").unwrap();
/// parse_all::<ast::LitNumber>("42.42").unwrap();
/// parse_all::<ast::LitNumber>("0.42").unwrap();
/// parse_all::<ast::LitNumber>("0.42e10").unwrap();
/// ```
impl Parse for LitNumber {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        Ok(match token.kind {
            ast::Kind::LitNumber(source) => LitNumber { source, token },
            _ => {
                return Err(ParseError::new(
                    token,
                    ParseErrorKind::ExpectedNumber { actual: token.kind },
                ));
            }
        })
    }
}

impl<'a> Resolve<'a> for LitNumber {
    type Output = ast::Number;

    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<ast::Number, ParseError> {
        use num::{Num as _, ToPrimitive as _};
        use std::ops::Neg as _;
        use std::str::FromStr as _;

        let span = self.token.span();

        let text = match self.source {
            ast::NumberSource::Synthetic(id) => match storage.get_number(id) {
                Some(number) => return Ok(number),
                None => {
                    return Err(ParseError::new(
                        span,
                        ParseErrorKind::BadSyntheticId { kind: "number", id },
                    ));
                }
            },
            ast::NumberSource::Text(text) => text,
        };

        let string = source
            .source(span)
            .ok_or_else(|| ParseError::new(span, ParseErrorKind::BadSlice))?;

        let string = if text.is_negative {
            &string[1..]
        } else {
            string
        };

        if text.is_fractional {
            let number = f64::from_str(string).map_err(err_span(span))?;
            return Ok(ast::Number::Float(number));
        }

        let (s, radix) = match text.base {
            ast::NumberBase::Binary => (2, 2),
            ast::NumberBase::Octal => (2, 8),
            ast::NumberBase::Hex => (2, 16),
            ast::NumberBase::Decimal => (0, 10),
        };

        let number = num::BigUint::from_str_radix(&string[s..], radix).map_err(err_span(span))?;

        let number = if text.is_negative {
            num::BigInt::from(number).neg().to_i64()
        } else {
            number.to_i64()
        };

        let number = match number {
            Some(n) => n,
            None => return Err(ParseError::new(span, ParseErrorKind::BadNumberOutOfBounds)),
        };

        return Ok(ast::Number::Integer(number));

        fn err_span<E>(span: Span) -> impl Fn(E) -> ParseError {
            move |_| ParseError::new(span, ParseErrorKind::BadNumberLiteral)
        }
    }
}
