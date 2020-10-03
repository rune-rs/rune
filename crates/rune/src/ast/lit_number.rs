use crate::ast;
use crate::{
    Parse, ParseError, ParseErrorKind, Parser, Resolve, ResolveOwned, Spanned, Storage, ToTokens,
};
use runestick::{Source, Span};

/// A number literal.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct LitNumber {
    /// The token corresponding to the literal.
    pub token: ast::Token,
    /// The source of the number.
    #[rune(skip)]
    pub source: ast::NumberSource,
}

/// Parse a number literal.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::LitNumber>("42");
/// testing::roundtrip::<ast::LitNumber>("42.42");
/// testing::roundtrip::<ast::LitNumber>("0.42");
/// testing::roundtrip::<ast::LitNumber>("0.42e10");
/// ```
impl Parse for LitNumber {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        match token.kind {
            ast::Kind::LitNumber(source) => Ok(LitNumber { source, token }),
            _ => Err(ParseError::expected(token, "number")),
        }
    }
}

impl<'a> Resolve<'a> for LitNumber {
    type Output = ast::Number;

    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<ast::Number, ParseError> {
        use num::Num as _;
        use std::str::FromStr as _;

        let span = self.token.span();

        let text = match self.source {
            ast::NumberSource::Synthetic(id) => match storage.get_number(id) {
                Some(number) => return Ok(number.clone()),
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

        let number = num::BigInt::from_str_radix(&string[s..], radix).map_err(err_span(span))?;
        return Ok(ast::Number::Integer(number));

        fn err_span<E>(span: Span) -> impl Fn(E) -> ParseError {
            move |_| ParseError::new(span, ParseErrorKind::BadNumberLiteral)
        }
    }
}

impl ResolveOwned for LitNumber {
    type Owned = ast::Number;

    fn resolve_owned(&self, storage: &Storage, source: &Source) -> Result<Self::Owned, ParseError> {
        self.resolve(storage, source)
    }
}
