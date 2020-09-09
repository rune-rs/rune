use crate::ast;
use crate::{Parse, ParseError, Parser, Peek, Resolve, Storage};
use runestick::{Source, Span};
use std::borrow::Cow;

/// A label, like `'foo`
#[derive(Debug, Clone, Copy)]
pub struct Label {
    /// The token of the label.
    pub token: ast::Token,
    /// The kind of the label.
    pub kind: ast::StringSource,
}

impl Label {
    /// Access the span of the identifier.
    pub fn span(&self) -> Span {
        self.token.span
    }
}

impl Parse for Label {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        match token.kind {
            ast::Kind::Label(kind) => Ok(Self { token, kind }),
            _ => Err(ParseError::TokenMismatch {
                expected: ast::Kind::Label(ast::StringSource::Text),
                actual: token.kind,
                span: token.span,
            }),
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
        let span = self.token.span;

        match self.kind {
            ast::StringSource::Text => {
                let span = self.token.span;

                let ident = source
                    .source(span.trim_start(1))
                    .ok_or_else(|| ParseError::BadSlice { span })?;

                Ok(Cow::Borrowed(ident))
            }
            ast::StringSource::Synthetic(id) => {
                let ident = storage
                    .get_string(id)
                    .ok_or_else(|| ParseError::BadSyntheticId {
                        kind: "ident",
                        id,
                        span,
                    })?;

                Ok(Cow::Owned(ident))
            }
        }
    }
}

impl crate::IntoTokens for Label {
    fn into_tokens(&self, _: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        stream.push(self.token);
    }
}
