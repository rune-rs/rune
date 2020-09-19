use crate::ast;
use crate::{Ast, Parse, ParseError, ParseErrorKind, Parser, Peek, Spanned};

/// A declaration.
#[derive(Debug, Clone, Ast, Spanned)]
pub enum Item {
    /// A use declaration.
    ItemUse(ast::ItemUse),
    /// A function declaration.
    // large variant, so boxed
    ItemFn(Box<ast::ItemFn>),
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

impl Item {
    /// Indicates if the declaration needs a semi-colon or not.
    pub fn needs_semi_colon(&self) -> bool {
        matches!(self, Self::MacroCall(..))
    }

    /// Test if declaration is suitable inside of a block.
    pub fn peek_as_stmt(parser: &mut Parser<'_>) -> Result<bool, ParseError> {
        let t1 = parser.token_peek_pair()?;
        let (t1, t2) = peek!(t1, Ok(false));

        Ok(match t1.kind {
            ast::Kind::Use => true,
            ast::Kind::Enum => true,
            ast::Kind::Struct => true,
            ast::Kind::Impl => true,
            ast::Kind::Async => matches!(peek!(t2, Ok(false)).kind, ast::Kind::Fn),
            ast::Kind::Fn => true,
            ast::Kind::Mod => true,
            _ => false,
        })
    }

    /// Parse an item within a nested block
    pub fn parse_in_nested_block(parser: &mut Parser) -> Result<Self, ParseError> {
        let attributes: Vec<ast::Attribute> = parser.parse()?;
        let t = parser.token_peek_eof()?;

        Ok(match t.kind {
            ast::Kind::Use => {
                Self::ItemUse(ast::ItemUse::parse_with_attributes(parser, attributes)?)
            }
            ast::Kind::Enum => {
                Self::ItemEnum(ast::ItemEnum::parse_with_attributes(parser, attributes)?)
            }
            ast::Kind::Struct => {
                Self::ItemStruct(ast::ItemStruct::parse_with_attributes(parser, attributes)?)
            }
            ast::Kind::Impl => {
                Self::ItemImpl(ast::ItemImpl::parse_with_attributes(parser, attributes)?)
            }
            ast::Kind::Async | ast::Kind::Fn => Self::ItemFn(Box::new(
                ast::ItemFn::parse_with_attributes(parser, attributes)?,
            )),
            ast::Kind::Mod => {
                Self::ItemMod(ast::ItemMod::parse_with_attributes(parser, attributes)?)
            }
            ast::Kind::Ident(..) => Self::MacroCall(parser.parse()?),
            _ => {
                return Err(ParseError::new(
                    t,
                    ParseErrorKind::ExpectedItem { actual: t.kind },
                ))
            }
        })
    }
}

impl Peek for Item {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        match peek!(t1).kind {
            ast::Kind::Use => true,
            ast::Kind::Enum => true,
            ast::Kind::Struct => true,
            ast::Kind::Impl => true,
            ast::Kind::Async => matches!(peek!(t2).kind, ast::Kind::Fn),
            ast::Kind::Fn => true,
            ast::Kind::Mod => true,
            ast::Kind::Ident(..) => true,
            _ => ast::Attribute::peek(t1, t2),
        }
    }
}

impl Parse for Item {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let attributes: Vec<ast::Attribute> = parser.parse()?;
        let t = parser.token_peek_eof()?;

        Ok(match t.kind {
            ast::Kind::Use => {
                Self::ItemUse(ast::ItemUse::parse_with_attributes(parser, attributes)?)
            }
            ast::Kind::Enum => {
                Self::ItemEnum(ast::ItemEnum::parse_with_attributes(parser, attributes)?)
            }
            ast::Kind::Struct => {
                Self::ItemStruct(ast::ItemStruct::parse_with_attributes(parser, attributes)?)
            }
            ast::Kind::Impl => {
                Self::ItemImpl(ast::ItemImpl::parse_with_attributes(parser, attributes)?)
            }
            ast::Kind::Async | ast::Kind::Fn => Self::ItemFn(Box::new(
                ast::ItemFn::parse_with_attributes(parser, attributes)?,
            )),
            ast::Kind::Mod => {
                Self::ItemMod(ast::ItemMod::parse_with_attributes(parser, attributes)?)
            }
            ast::Kind::Ident(..) => Self::MacroCall(parser.parse()?),
            _ => {
                return Err(ParseError::new(
                    t,
                    ParseErrorKind::ExpectedItem { actual: t.kind },
                ))
            }
        })
    }
}
