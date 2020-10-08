use crate::ast;
use crate::attrs::Attribute;
use crate::macros::Storage;
use crate::parsing::{Parse, ParseError, ParseErrorKind, Parser, Resolve as _};
use crate::Spanned as _;
use runestick::Source;
use runestick::Span;
use std::collections::BTreeSet;
use std::sync::Arc;

/// Helper for parsing internal attributes.
pub(crate) struct Attributes {
    /// Collection of attributes that have been used.
    unused: BTreeSet<usize>,
    /// All raw attributes.
    attributes: Vec<ast::Attribute>,
    storage: Storage,
    source: Arc<Source>,
}

impl Attributes {
    /// Construct a new attriutes parser.
    pub(crate) fn new(
        attributes: Vec<ast::Attribute>,
        storage: Storage,
        source: Arc<Source>,
    ) -> Self {
        Self {
            unused: attributes.iter().enumerate().map(|(i, _)| i).collect(),
            attributes,
            storage,
            source,
        }
    }

    /// Try to parse the attribute with the given type.
    pub(crate) fn try_parse<T>(&mut self) -> Result<Option<T>, ParseError>
    where
        T: Attribute + Parse,
    {
        let mut matched = None;

        for index in self.unused.iter().copied() {
            let a = match self.attributes.get(index) {
                Some(a) => a,
                None => continue,
            };

            let ident = match a.path.try_as_ident() {
                Some(ident) => ident,
                None => continue,
            };

            let ident = ident.resolve(&self.storage, &self.source)?;

            if ident != T::PATH {
                continue;
            }

            if matched.is_some() {
                return Err(ParseError::new(
                    a.span(),
                    ParseErrorKind::MultipleMatchingAttributes { name: T::PATH },
                ));
            }

            let mut parser = Parser::from_token_stream(&a.input);
            matched = Some((index, parser.parse::<T>()?));
            parser.eof()?;
        }

        if let Some((index, matched)) = matched {
            self.unused.remove(&index);
            Ok(Some(matched))
        } else {
            Ok(None)
        }
    }

    /// Get the span of the first remaining attribute.
    pub(crate) fn remaining(&self) -> Option<Span> {
        for i in self.unused.iter().copied() {
            if let Some(a) = self.attributes.get(i) {
                return Some(a.span());
            }
        }

        None
    }
}
