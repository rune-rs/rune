use core::marker::PhantomData;

use crate::no_std::collections::VecDeque;
use crate::no_std::prelude::*;

use crate as rune;
use crate::ast;
use crate::ast::{LitStr, Spanned};
use crate::compile::{self, ParseErrorKind};
use crate::parse::{self, Parse, Resolve, ResolveContext};

/// Helper for parsing internal attributes.
pub(crate) struct Parser {
    /// Collection of attributes that have been used.
    unused: VecDeque<usize>,
    /// Attributes which were missed during the last parse.
    missed: Vec<usize>,
}

impl Parser {
    /// Construct a new attributes parser.
    pub(crate) fn new(attributes: &[ast::Attribute]) -> Self {
        Self {
            unused: attributes.iter().enumerate().map(|(i, _)| i).collect(),
            missed: Vec::new(),
        }
    }

    /// Try to parse and collect all attributes of a given type.
    ///
    /// The returned Vec may be empty.
    pub(crate) fn parse_all<'this, 'a, T>(
        &'this mut self,
        cx: ResolveContext<'this>,
        attributes: &'a [ast::Attribute],
    ) -> ParseAll<'this, 'a, T>
    where
        T: Attribute + Parse,
    {
        for index in self.missed.drain(..) {
            self.unused.push_back(index);
        }

        ParseAll {
            outer: self,
            attributes,
            cx,
            _marker: PhantomData,
        }
    }

    /// Try to parse a unique attribute with the given type.
    ///
    /// Returns the parsed element and the span it was parsed from if
    /// successful.
    pub(crate) fn try_parse<'a, T>(
        &mut self,
        cx: ResolveContext<'_>,
        attributes: &'a [ast::Attribute],
    ) -> compile::Result<Option<(&'a ast::Attribute, T)>>
    where
        T: Attribute + Parse,
    {
        let mut vec = self.parse_all::<T>(cx, attributes);
        let first = vec.next();
        let second = vec.next();

        match (first, second) {
            (None, _) => Ok(None),
            (Some(first), None) => Ok(Some(first?)),
            (Some(first), _) => Err(compile::Error::new(
                first?.0,
                ParseErrorKind::MultipleMatchingAttributes { name: T::PATH },
            )),
        }
    }

    /// Get the span of the first remaining attribute.
    pub(crate) fn remaining<'a>(
        &'a self,
        attributes: &'a [ast::Attribute],
    ) -> impl Iterator<Item = &ast::Attribute> + 'a {
        self.unused
            .iter()
            .chain(self.missed.iter())
            .flat_map(|&n| attributes.get(n))
    }
}

pub(crate) struct ParseAll<'this, 'a, T> {
    outer: &'this mut Parser,
    attributes: &'a [ast::Attribute],
    cx: ResolveContext<'this>,
    _marker: PhantomData<T>,
}

impl<'this, 'a, T> Iterator for ParseAll<'this, 'a, T>
where
    T: Attribute + Parse,
{
    type Item = compile::Result<(&'a ast::Attribute, T)>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let index = self.outer.unused.pop_front()?;

            let Some(a) = self.attributes.get(index) else {
                self.outer.missed.push(index);
                continue;
            };

            let Some(ident) = a.path.try_as_ident() else {
                self.outer.missed.push(index);
                continue;
            };

            let ident = match ident.resolve(self.cx) {
                Ok(ident) => ident,
                Err(e) => {
                    return Some(Err(e));
                }
            };

            if ident != T::PATH {
                self.outer.missed.push(index);
                continue;
            }

            let mut parser = parse::Parser::from_token_stream(&a.input, a.span());

            let item = match parser.parse::<T>() {
                Ok(item) => item,
                Err(e) => {
                    return Some(Err(e));
                }
            };

            if let Err(e) = parser.eof() {
                return Some(Err(e));
            }

            return Some(Ok((a, item)));
        }
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
    pub(crate) fn args(&self, cx: ResolveContext<'_>) -> compile::Result<BuiltInArgs> {
        let mut out = BuiltInArgs::default();

        if let Some(args) = &self.args {
            for (ident, _) in args {
                match ident.resolve(cx)? {
                    "literal" => {
                        out.literal = true;
                    }
                    _ => {
                        return Err(compile::Error::msg(ident, "unsupported attribute"));
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

#[derive(Parse)]
pub(crate) struct Doc {
    /// The `=` token.
    #[allow(dead_code)]
    pub eq_token: T![=],
    /// The doc string.
    pub doc_string: LitStr,
}

impl Attribute for Doc {
    /// Must match the specified name.
    const PATH: &'static str = "doc";
}
