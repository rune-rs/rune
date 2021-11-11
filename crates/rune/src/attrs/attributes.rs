use crate::ast;
use crate::attrs::Attribute;
use crate::macros::Storage;
use crate::parsing::{Parse, ParseError, ParseErrorKind, Parser, Resolve as _};
use crate::{Sources, Spanned as _};
use runestick::Span;
use std::collections::BTreeSet;

/// Helper for parsing internal attributes.
pub(crate) struct Attributes {
    /// Collection of attributes that have been used.
    unused: BTreeSet<usize>,
    /// All raw attributes.
    attributes: Vec<ast::Attribute>,
}

impl Attributes {
    /// Construct a new attriutes parser.
    pub(crate) fn new(attributes: Vec<ast::Attribute>) -> Self {
        Self {
            unused: attributes.iter().enumerate().map(|(i, _)| i).collect(),
            attributes,
        }
    }

    /// Drain all attributes under the assumption that they have been validated
    /// elsewhere.
    pub(crate) fn drain(&mut self) {
        self.unused.clear();
    }

    /// Try to parse the attribute with the given type.
    ///
    /// Returns the parsed element and the span it was parsed from if
    /// successful.
    pub(crate) fn try_parse<T>(
        &mut self,
        storage: &Storage,
        sources: &Sources,
    ) -> Result<Option<(Span, T)>, ParseError>
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

            let ident = ident.resolve(storage, sources)?;

            if ident != T::PATH {
                continue;
            }

            let span = a.span();

            if matched.is_some() {
                return Err(ParseError::new(
                    span,
                    ParseErrorKind::MultipleMatchingAttributes { name: T::PATH },
                ));
            }

            let mut parser = Parser::from_token_stream(&a.input, a.span());
            matched = Some((index, span, parser.parse::<T>()?));
            parser.eof()?;
        }

        if let Some((index, span, matched)) = matched {
            self.unused.remove(&index);
            Ok(Some((span, matched)))
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
