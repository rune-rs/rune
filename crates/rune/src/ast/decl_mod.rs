use crate::ast;
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::{Parse, Peek};
use runestick::Span;

/// A module declaration.
#[derive(Debug, Clone)]
pub struct DeclMod {
    /// The `mod` keyword.
    pub mod_: ast::Mod,
    /// The name of the mod.
    pub name: ast::Ident,
    /// The optional body of the module declaration.
    pub body: Option<DeclModBody>,
}

impl DeclMod {
    /// The span of the declaration.
    pub fn span(&self) -> Span {
        if let Some(body) = &self.body {
            self.mod_.span().join(body.span())
        } else {
            self.mod_.span().join(self.name.span())
        }
    }

    /// If the declaration needs a semi-colon or not.
    pub fn needs_semi_colon(&self) -> bool {
        self.body.is_none()
    }
}

impl Parse for DeclMod {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            mod_: parser.parse()?,
            name: parser.parse()?,
            body: parser.parse()?,
        })
    }
}

/// A module declaration.
#[derive(Debug, Clone)]
pub struct DeclModBody {
    /// The open brace.
    pub open: ast::OpenBrace,
    /// A nested "file" declaration.
    pub file: Box<ast::DeclFile>,
    /// The close brace.
    pub close: ast::CloseBrace,
}

impl DeclModBody {
    /// The span of the body.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }
}

impl Parse for DeclModBody {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            open: parser.parse()?,
            file: parser.parse()?,
            close: parser.parse()?,
        })
    }
}

impl Peek for DeclModBody {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        ast::OpenBrace::peek(t1, t2)
    }
}
