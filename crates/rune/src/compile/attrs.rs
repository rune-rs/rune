use crate::ast;
use crate::ast::{Span, Spanned};
use crate::parse::{Parse, ParseError, ParseErrorKind, Parser, Resolve, ResolveContext};
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
        ctx: ResolveContext<'_>,
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

            let ident = ident.resolve(ctx)?;

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

pub(crate) trait Attribute {
    const PATH: &'static str;
}

#[derive(Default)]
pub(crate) struct BuiltInArgs {
    pub(crate) literal: bool,
}

#[derive(Parse)]
pub(crate) struct BuiltIn {
    /// Arguments to this built-in.
    pub args: Option<ast::Parenthesized<ast::Ident, T![,]>>,
}

impl BuiltIn {
    /// Parse built-in arguments.
    pub(crate) fn args(&self, ctx: ResolveContext<'_>) -> Result<BuiltInArgs, ParseError> {
        let mut out = BuiltInArgs::default();

        if let Some(args) = &self.args {
            for (ident, _) in args {
                match ident.resolve(ctx)? {
                    "literal" => {
                        out.literal = true;
                    }
                    _ => {
                        return Err(ParseError::msg(ident, "unsupported attribute"));
                    }
                }
            }
        }

        Ok(out)
    }
}

impl Attribute for BuiltIn {
    /// Must match the specified name.
    const PATH: &'static str = "builtin";
}

/// NB: at this point we don't support attributes beyond the empty `#[test]`.
#[derive(Parse)]
pub(crate) struct Test {}

impl Attribute for Test {
    /// Must match the specified name.
    const PATH: &'static str = "test";
}

/// NB: at this point we don't support attributes beyond the empty `#[bench]`.
#[derive(Parse)]
pub(crate) struct Bench {}

impl Attribute for Bench {
    /// Must match the specified name.
    const PATH: &'static str = "bench";
}
