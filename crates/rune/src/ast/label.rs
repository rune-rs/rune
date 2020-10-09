use crate::ast;
use crate::{
    Parse, ParseError, ParseErrorKind, Parser, Peek, Peeker, Resolve, ResolveOwned, Spanned,
    Storage, ToTokens,
};
use runestick::Source;
use std::borrow::Cow;

/// A label, like `'foo`
#[derive(Debug, Clone, Copy, PartialEq, Eq, ToTokens, Spanned)]
pub struct Label {
    /// The token of the label.
    pub token: ast::Token,
    /// The kind of the label.
    #[rune(skip)]
    pub source: ast::StringSource,
}

impl Label {
    /// Construct a new synthetic label. The label should be specified without
    /// the leading `'`, so `hello` instead of `'hello`.
    ///
    /// # Panics
    ///
    /// This will panic if it's called outside of a macro context.
    pub fn new(label: &str) -> Self {
        crate::macros::current_context(|ctx| ctx.label(label))
    }
}

impl Parse for Label {
    fn parse(p: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = p.next()?;

        match token.kind {
            K!['label(source)] => Ok(Self { token, source }),
            _ => Err(ParseError::expected(&token, "label")),
        }
    }
}

impl Peek for Label {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!(p.nth(0), K!['label])
    }
}

impl<'a> Resolve<'a> for Label {
    type Output = Cow<'a, str>;

    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<Cow<'a, str>, ParseError> {
        let span = self.token.span();

        match self.source {
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
            ast::StringSource::BuiltIn(builtin) => Ok(Cow::Borrowed(builtin.as_str())),
        }
    }
}

impl ResolveOwned for Label {
    type Owned = String;

    fn resolve_owned(&self, storage: &Storage, source: &Source) -> Result<Self::Owned, ParseError> {
        Ok(self.resolve(storage, source)?.into_owned())
    }
}
