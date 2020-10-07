use crate::ast;
use crate::{Parse, ParseError, ParseErrorKind, Parser, Spanned, ToTokens, TokenStream};

/// A function call `<expr>!(<args>)`.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct MacroCall {
    /// Attributes associated with macro call.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The expression being called over.
    pub path: ast::Path,
    /// Bang operator `!`.
    pub bang: T![!],
    /// Opening token.
    pub open: ast::Token,
    /// The tokens provided to the macro.
    #[rune(optional)]
    pub stream: TokenStream,
    /// Closing token.
    pub close: ast::Token,
}

impl MacroCall {
    /// Parse with an expression.
    pub fn parse_with_meta_path(
        parser: &mut Parser,
        attributes: Vec<ast::Attribute>,
        path: ast::Path,
    ) -> Result<Self, ParseError> {
        let bang = parser.parse()?;

        let mut level = 1;
        let open = parser.next()?;

        let delim = match open.kind {
            ast::Kind::Open(delim) => delim,
            _ => {
                return Err(ParseError::expected(
                    open,
                    "macro call delimiter `(`, `[`, or `{`",
                ));
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
                            return Err(ParseError::new(
                                open,
                                ParseErrorKind::ExpectedMacroCloseDelimiter {
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

            stream.push(token);
        }

        Ok(Self {
            attributes,
            bang,
            path,
            open,
            stream: TokenStream::from(stream),
            close,
        })
    }
}

impl Parse for MacroCall {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        let path = parser.parse()?;
        Self::parse_with_meta_path(parser, attributes, path)
    }
}
