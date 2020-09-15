use crate::ast;
use crate::{IntoTokens, Parse, ParseError, Parser, Peek, Spanned};
use runestick::Span;

/// A module declaration.
#[derive(Debug, Clone)]
pub struct ItemMod {
    /// The `mod` keyword.
    pub mod_: ast::Mod,
    /// The name of the mod.
    pub name: ast::Ident,
    /// The optional body of the module declaration.
    pub body: ItemModBody,
}

impl Spanned for ItemMod {
    fn span(&self) -> Span {
        self.mod_.span().join(self.body.span())
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

/// An item body.
#[derive(Debug, Clone)]
pub enum ItemModBody {
    /// An empty body terminated by a semicolon.
    EmptyBody(ast::SemiColon),
    /// An inline body.
    InlineBody(ItemInlineBody),
}

impl ItemModBody {
    /// Get the span of the mod body.
    pub fn span(&self) -> Span {
        match self {
            Self::EmptyBody(semi) => semi.span(),
            Self::InlineBody(body) => body.span(),
        }
    }
}

impl Parse for ItemModBody {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let t = parser.token_peek_eof()?;

        Ok(match t.kind {
            ast::Kind::Open(ast::Delimiter::Brace) => Self::InlineBody(parser.parse()?),
            _ => Self::EmptyBody(parser.parse()?),
        })
    }
}

impl IntoTokens for ItemModBody {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        match self {
            Self::EmptyBody(semi) => semi.into_tokens(context, stream),
            Self::InlineBody(body) => body.into_tokens(context, stream),
        }
    }
}

/// A module declaration.
#[derive(Debug, Clone)]
pub struct ItemInlineBody {
    /// The open brace.
    pub open: ast::OpenBrace,
    /// A nested "file" declaration.
    pub file: Box<ast::File>,
    /// The close brace.
    pub close: ast::CloseBrace,
}

impl ItemInlineBody {
    /// The span of the body.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }
}

impl Parse for ItemInlineBody {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            open: parser.parse()?,
            file: parser.parse()?,
            close: parser.parse()?,
        })
    }
}

impl Peek for ItemInlineBody {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        ast::OpenBrace::peek(t1, t2)
    }
}

impl IntoTokens for ItemInlineBody {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        self.open.into_tokens(context, stream);
        self.file.into_tokens(context, stream);
        self.close.into_tokens(context, stream);
    }
}
