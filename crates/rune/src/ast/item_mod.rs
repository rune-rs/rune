use crate::ast;
use crate::{IntoTokens, Parse, ParseError, Parser, Peek};
use runestick::Span;

/// A module declaration.
#[derive(Debug, Clone)]
pub struct ItemMod {
    /// The `mod` keyword.
    pub mod_: ast::Mod,
    /// The name of the mod.
    pub name: ast::Ident,
    /// The optional body of the module declaration.
    pub body: Option<ItemModBody>,
}

impl ItemMod {
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

impl Parse for ItemMod {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            mod_: parser.parse()?,
            name: parser.parse()?,
            body: parser.parse()?,
        })
    }
}

impl IntoTokens for ItemMod {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        self.mod_.into_tokens(context, stream);
        self.name.into_tokens(context, stream);
        self.body.into_tokens(context, stream);
    }
}

/// A module declaration.
#[derive(Debug, Clone)]
pub struct ItemModBody {
    /// The open brace.
    pub open: ast::OpenBrace,
    /// A nested "file" declaration.
    pub file: Box<ast::File>,
    /// The close brace.
    pub close: ast::CloseBrace,
}

impl ItemModBody {
    /// The span of the body.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }
}

impl Parse for ItemModBody {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            open: parser.parse()?,
            file: parser.parse()?,
            close: parser.parse()?,
        })
    }
}

impl Peek for ItemModBody {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        ast::OpenBrace::peek(t1, t2)
    }
}

impl IntoTokens for ItemModBody {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        self.open.into_tokens(context, stream);
        self.file.into_tokens(context, stream);
        self.close.into_tokens(context, stream);
    }
}
