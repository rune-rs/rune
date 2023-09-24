use super::prelude::*;
use super::Eq;

/// An `= ...` e.g. inside an attribute `#[doc = ...]`.
///
/// To get unparsed tokens use `EqValue<TokenStream>`.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Parse, Spanned)]
#[try_clone(bound = {T: TryClone})]
pub struct EqValue<T> {
    /// The `=` token.
    pub eq: Eq,
    /// The remainder.
    pub value: T,
}

/// Parses `[{( ... )}]` ensuring that the delimiter is balanced.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
pub struct Group {
    /// The opening delimiter.
    pub open: ast::Token,
    /// The content between the delimiters.
    #[rune(iter)]
    pub content: TokenStream,
    /// The closing delimiter.
    pub close: ast::Token,
}

impl Parse for Group {
    fn parse(parser: &mut Parser) -> compile::Result<Self> {
        let mut level = 1;
        let open = parser.next()?;

        let delim = match open.kind {
            ast::Kind::Open(delim) => delim,
            _ => {
                return Err(compile::Error::expected(open, Expectation::OpenDelimiter));
            }
        };

        let close;

        let mut stream = Vec::new();

        loop {
            let token = parser.next()?;

            match token.kind {
                ast::Kind::Open(..) => level += 1,
                ast::Kind::Close(actual) => {
                    level -= 1;

                    if level == 0 {
                        if actual != delim {
                            return Err(compile::Error::new(
                                open,
                                ErrorKind::ExpectedMacroCloseDelimiter {
                                    actual: token.kind,
                                    expected: ast::Kind::Close(delim),
                                },
                            ));
                        }

                        close = token;
                        break;
                    }
                }
                _ => (),
            }

            stream.try_push(token)?;
        }

        Ok(Self {
            open,
            content: TokenStream::from(stream),
            close,
        })
    }
}
