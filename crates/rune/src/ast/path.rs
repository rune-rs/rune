use crate::ast::{Ident, Scope};
use crate::error::{ParseError, ResolveError, Result};
use crate::parser::Parser;
use crate::source::Source;
use crate::traits::{Parse, Resolve};
use st::unit::Span;

/// A path, where each element is separated by a `::`.
#[derive(Debug, Clone)]
pub struct Path {
    /// The first component in the path.
    pub first: Ident,
    /// The rest of the components in the path.
    pub rest: Vec<(Scope, Ident)>,
}

impl Path {
    /// Convert into an identifier used for instance calls.
    pub fn into_instance_call_ident(self) -> Result<Ident, ParseError> {
        if !self.rest.is_empty() {
            return Err(ParseError::PathCallInstanceError { span: self.span() });
        }

        Ok(self.first)
    }

    /// Calculate the full span of the path.
    pub fn span(&self) -> Span {
        match self.rest.last() {
            Some((_, ident)) => self.first.span().join(ident.span()),
            None => self.first.span(),
        }
    }
}

impl Parse for Path {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let first = parser.parse()?;
        let mut rest = Vec::new();

        while parser.peek::<Scope>()? {
            let scope = parser.parse::<Scope>()?;
            rest.push((scope, parser.parse()?));
        }

        Ok(Self { first, rest })
    }
}

impl<'a> Resolve<'a> for Path {
    type Output = Vec<&'a str>;

    fn resolve(&self, source: Source<'a>) -> Result<Vec<&'a str>, ResolveError> {
        let mut output = Vec::new();

        output.push(self.first.resolve(source)?);

        for (_, ident) in &self.rest {
            output.push(ident.resolve(source)?);
        }

        Ok(output)
    }
}
