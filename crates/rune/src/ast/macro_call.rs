use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::MacroCall>("foo!()");
    rt::<ast::MacroCall>("::bar::foo!(question to life)");
}

/// A macro call.
///
/// * `<expr>!(<args>)`.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned, Opaque)]
#[non_exhaustive]
pub struct MacroCall {
    /// Opaque identifier for macro call. Use to store reference to internally
    /// expanded macros.
    #[rune(id)]
    pub(crate) id: Id,
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
    #[rune(iter)]
    pub input: TokenStream,
    /// Closing token.
    pub close: ast::Token,
}

impl MacroCall {
    /// Test if macro needs semi or not.
    pub(crate) fn needs_semi(&self) -> bool {
        !matches!(self.close.kind, K!['}'])
    }

    /// The span of the input token stream.
    pub(crate) fn input_span(&self) -> Span {
        if let Some(span) = self.input.option_span() {
            span
        } else {
            self.open.span.tail()
        }
    }

    /// Parse with an expression.
    pub(crate) fn parse_with_meta_path(
        parser: &mut Parser,
        attributes: Vec<ast::Attribute>,
        path: ast::Path,
    ) -> Result<Self> {
        let bang = parser.parse()?;

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
            id: Default::default(),
            attributes,
            bang,
            path,
            open,
            input: TokenStream::from(stream),
            close,
        })
    }
}

impl Parse for MacroCall {
    fn parse(parser: &mut Parser) -> Result<Self> {
        let attributes = parser.parse()?;
        let path = parser.parse()?;
        Self::parse_with_meta_path(parser, attributes, path)
    }
}
