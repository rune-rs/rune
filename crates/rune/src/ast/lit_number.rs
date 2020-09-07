use crate::ast;
use crate::{IntoTokens, Parse, ParseError, Parser, Resolve, Storage};
use runestick::{Source, Span};

/// A resolved number literal.
pub enum Number {
    /// A float literal number.
    Float(f64),
    /// An integer literal number.
    Integer(i64),
}

/// A number literal.
#[derive(Debug, Clone)]
pub struct LitNumber {
    /// If the number is negative.
    is_negative: bool,
    /// Indicates if the number is fractional.
    is_fractional: bool,
    /// The kind of the number literal.
    number: ast::NumberKind,
    /// The token corresponding to the literal.
    token: ast::Token,
}

impl LitNumber {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.token.span
    }
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
            ast::Kind::LitNumber {
                is_negative,
                is_fractional,
                number,
                ..
            } => LitNumber {
                is_negative,
                is_fractional,
                number,
                token,
            },
            _ => {
                return Err(ParseError::ExpectedNumber {
                    actual: token.kind,
                    span: token.span,
                })
            }
        })
    }
}

impl<'a> Resolve<'a> for LitNumber {
    type Output = Number;

    fn resolve(&self, _: &Storage, source: &'a Source) -> Result<Number, ParseError> {
        use num::{Num as _, ToPrimitive as _};
        use std::ops::Neg as _;
        use std::str::FromStr as _;

        let span = self.token.span;

        let string = source
            .source(span)
            .ok_or_else(|| ParseError::BadSlice { span })?;

        let string = if self.is_negative {
            &string[1..]
        } else {
            string
        };

        if self.is_fractional {
            let number = f64::from_str(string).map_err(err_span(span))?;
            return Ok(Number::Float(number));
        }

        let (s, radix) = match self.number {
            ast::NumberKind::Binary => (2, 2),
            ast::NumberKind::Octal => (2, 8),
            ast::NumberKind::Hex => (2, 16),
            ast::NumberKind::Decimal => (0, 10),
        };

        let number = num::BigUint::from_str_radix(&string[s..], radix).map_err(err_span(span))?;

        let number = if self.is_negative {
            num::BigInt::from(number).neg().to_i64()
        } else {
            number.to_i64()
        };

        let number = match number {
            Some(n) => n,
            None => return Err(ParseError::BadNumberOutOfBounds { span }),
        };

        return Ok(Number::Integer(number));

        fn err_span<E>(span: Span) -> impl Fn(E) -> ParseError {
            move |_| ParseError::BadNumberLiteral { span }
        }
    }
}

impl IntoTokens for LitNumber {
    fn into_tokens(&self, _: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        stream.push(self.token);
    }
}
