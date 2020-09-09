use crate::ast;
use crate::parser::Parser;
use crate::traits::{Parse, Peek};
use crate::ParseError;
use runestick::Span;

/// A declaration.
#[derive(Debug, Clone)]
pub enum Item {
    /// A use declaration.
    ItemUse(ast::ItemUse),
    /// A function declaration.
    // large size difference between variants
    // we should box this variant.
    // https://rust-lang.github.io/rust-clippy/master/index.html#large_enum_variant
    ItemFn(ast::ItemFn),
    /// An enum declaration.
    ItemEnum(ast::ItemEnum),
    /// A struct declaration.
    ItemStruct(ast::ItemStruct),
    /// An impl declaration.
    ItemImpl(ast::ItemImpl),
    /// A module declaration.
    ItemMod(ast::ItemMod),
    /// A macro call expanding into an item.
    MacroCall(ast::MacroCall),
}

into_tokens_enum!(Item {
    ItemUse,
    ItemFn,
    ItemEnum,
    ItemStruct,
    ItemImpl,
    ItemMod,
    MacroCall
});

impl Item {
    /// The span of the declaration.
    pub fn span(&self) -> Span {
        match self {
            Self::ItemUse(decl) => decl.span(),
            Self::ItemFn(decl) => decl.span(),
            Self::ItemEnum(decl) => decl.span(),
            Self::ItemStruct(decl) => decl.span(),
            Self::ItemImpl(decl) => decl.span(),
            Self::ItemMod(decl) => decl.span(),
            Self::MacroCall(expr) => expr.span(),
        }
    }

    /// Indicates if the declaration needs a semi-colon or not.
    pub fn needs_semi_colon(&self) -> bool {
        match self {
            Self::ItemUse(..) => true,
            Self::ItemFn(..) => false,
            Self::ItemEnum(..) => false,
            Self::ItemStruct(decl_struct) => decl_struct.needs_semi_colon(),
            Self::ItemImpl(..) => false,
            Self::ItemMod(decl_mod) => decl_mod.needs_semi_colon(),
            Self::MacroCall(..) => true,
        }
    }

    /// Test if declaration is suitable inside of a block.
    pub fn peek_as_stmt(parser: &mut Parser<'_>) -> Result<bool, ParseError> {
        let t1 = parser.token_peek()?;

        let t1 = match t1 {
            Some(t1) => t1,
            None => return Ok(false),
        };

        Ok(match t1.kind {
            ast::Kind::Use => true,
            ast::Kind::Enum => true,
            ast::Kind::Struct => true,
            ast::Kind::Impl => true,
            ast::Kind::Async | ast::Kind::Fn => true,
            ast::Kind::Mod => true,
            _ => false,
        })
    }
}

impl Peek for Item {
    fn peek(t1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        let t1 = match t1 {
            Some(t1) => t1,
            None => return false,
        };

        match t1.kind {
            ast::Kind::Use => true,
            ast::Kind::Enum => true,
            ast::Kind::Struct => true,
            ast::Kind::Impl => true,
            ast::Kind::Async | ast::Kind::Fn => true,
            ast::Kind::Mod => true,
            ast::Kind::Ident(..) => true,
            _ => false,
        }
    }
}

impl Parse for Item {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let t = parser.token_peek_eof()?;

        Ok(match t.kind {
            ast::Kind::Use => Self::ItemUse(parser.parse()?),
            ast::Kind::Enum => Self::ItemEnum(parser.parse()?),
            ast::Kind::Struct => Self::ItemStruct(parser.parse()?),
            ast::Kind::Impl => Self::ItemImpl(parser.parse()?),
            ast::Kind::Async | ast::Kind::Fn => Self::ItemFn(parser.parse()?),
            ast::Kind::Mod => Self::ItemMod(parser.parse()?),
            ast::Kind::Ident(..) => Self::MacroCall(parser.parse()?),
            _ => {
                return Err(ParseError::ExpectedItem {
                    actual: t.kind,
                    span: t.span,
                })
            }
        })
    }
}
