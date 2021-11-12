use crate::ast::prelude::*;

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
    /// Construct a new label from the given string. The string should be
    /// specified *without* the leading `'`, so `"foo"` instead of `"'foo"`.
    ///
    /// This constructor must only be used inside of a macro.
    pub fn new(ctx: &mut MacroContext<'_, '_>, label: &str) -> Self {
        Self::new_with(label, ctx.macro_span(), &mut ctx.q_mut().storage)
    }

    /// Construct a new label from the given string. The string should be
    /// specified *without* the leading `'`, so `"foo"` instead of `"'foo"`.
    ///
    /// This constructor does not panic when called outside of a macro context
    /// but requires access to a `span` and `storage`.
    pub fn new_with(label: &str, span: Span, storage: &mut Storage) -> Self {
        let id = storage.insert_str(label);
        let source = ast::StringSource::Synthetic(id);

        ast::Label {
            token: ast::Token {
                span,
                kind: ast::Kind::Label(source),
            },
            source,
        }
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
    type Output = &'a str;

    fn resolve(&self, storage: &'a Storage, sources: &'a Sources) -> Result<&'a str, ResolveError> {
        let span = self.token.span();

        match self.source {
            ast::StringSource::Text(source_id) => {
                let span = self.token.span();

                let ident = sources
                    .source(source_id, span.trim_start(1))
                    .ok_or_else(|| ResolveError::new(span, ResolveErrorKind::BadSlice))?;

                Ok(ident)
            }
            ast::StringSource::Synthetic(id) => {
                let ident = storage.get_string(id).ok_or_else(|| {
                    ResolveError::new(span, ResolveErrorKind::BadSyntheticId { kind: "ident", id })
                })?;

                Ok(ident)
            }
            ast::StringSource::BuiltIn(builtin) => Ok(builtin.as_str()),
        }
    }
}

impl ResolveOwned for Label {
    type Owned = String;

    fn resolve_owned(
        &self,
        storage: &Storage,
        sources: &Sources,
    ) -> Result<Self::Owned, ResolveError> {
        Ok(self.resolve(storage, sources)?.to_owned())
    }
}
