use crate::ast;
use crate::{Parse, ParseError, ParseErrorKind, Parser, Peek, Resolve, Spanned, Storage, ToTokens};
use runestick::Source;
use std::borrow::Cow;

/// A label, like `'foo`
#[derive(Debug, Clone, Copy, ToTokens, Spanned)]
pub struct Label {
    /// The token of the label.
    pub token: ast::Token,
    /// The kind of the label.
    #[rune(skip)]
    pub kind: ast::StringSource,
}

impl Parse for Label {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        match token.kind {
            ast::Kind::Label(kind) => Ok(Self { token, kind }),
            _ => Err(ParseError::new(
                token,
                ParseErrorKind::TokenMismatch {
                    expected: ast::Kind::Label(ast::StringSource::Text),
                    actual: token.kind,
                },
            )),
        }
    }
}

impl Peek for Label {
    fn peek(p1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        match p1 {
            Some(p1) => matches!(p1.kind, ast::Kind::Label(..)),
            _ => false,
        }
    }
}

impl<'a> Resolve<'a> for Label {
    type Output = Cow<'a, str>;

    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<Cow<'a, str>, ParseError> {
        let span = self.token.span();

        match self.kind {
            ast::StringSource::Text => {
                let span = self.token.span();

                let ident = source
                    .source(span.trim_start(1))
                    .ok_or_else(|| ParseError::new(span, ParseErrorKind::BadSlice))?;

                Ok(Cow::Borrowed(ident))
            }
            ast::StringSource::Synthetic(id) => {
                let ident = storage.get_string(id).ok_or_else(|| {
                    ParseError::new(span, ParseErrorKind::BadSyntheticId { kind: "ident", id })
                })?;

                Ok(Cow::Owned(ident))
            }
        }
    }
}
