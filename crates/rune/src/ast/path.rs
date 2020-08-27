use crate::ast;
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::source::Source;
use crate::token::{Kind, Token};
use crate::traits::{Parse, Peek, Resolve};
use runestick::unit::Span;

/// A path, where each element is separated by a `::`.
#[derive(Debug, Clone)]
pub struct Path {
    /// The first component in the path.
    pub first: ast::Ident,
    /// The rest of the components in the path.
    pub rest: Vec<(ast::Scope, ast::Ident)>,
}

impl Path {
    /// Convert into an identifier used for field access calls.
    ///
    /// This is only allowed if there are no other path components.
    pub fn try_into_ident(self) -> Option<ast::Ident> {
        if !self.rest.is_empty() {
            return None;
        }

        Some(self.first)
    }

    /// Calculate the full span of the path.
    pub fn span(&self) -> Span {
        match self.rest.last() {
            Some((_, ident)) => self.first.span().join(ident.span()),
            None => self.first.span(),
        }
    }

    /// Parse with the first identifier already parsed.
    pub fn parse_with_first(parser: &mut Parser, first: ast::Ident) -> Result<Self, ParseError> {
        let mut rest = Vec::new();

        while parser.peek::<ast::Scope>()? {
            let scope = parser.parse::<ast::Scope>()?;
            rest.push((scope, parser.parse()?));
        }

        Ok(Self { first, rest })
    }
}

impl Peek for Path {
    fn peek(t1: Option<Token>, _: Option<Token>) -> bool {
        let t1 = match t1 {
            Some(t1) => t1,
            None => return false,
        };

        matches!(t1.kind, Kind::Ident)
    }
}

impl Parse for Path {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let first = parser.parse()?;
        Self::parse_with_first(parser, first)
    }
}

impl<'a> Resolve<'a> for Path {
    type Output = Vec<&'a str>;

    fn resolve(&self, source: Source<'a>) -> Result<Vec<&'a str>, ParseError> {
        let mut output = Vec::new();

        output.push(self.first.resolve(source)?);

        for (_, ident) in &self.rest {
            output.push(ident.resolve(source)?);
        }

        Ok(output)
    }
}
