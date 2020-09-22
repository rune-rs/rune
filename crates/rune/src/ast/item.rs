use crate::ast;
use crate::{Parse, ParseError, ParseErrorKind, Parser, Spanned, ToTokens};

/// A declaration.
#[derive(Debug, Clone, ToTokens, Spanned)]
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
    /// A const declaration.
    ItemConst(ast::ItemConst),
    /// A macro call expanding into an item.
    MacroCall(ast::MacroCall),
}

impl Item {
    /// Indicates if the declaration needs a semi-colon or not.
    pub fn needs_semi_colon(&self) -> bool {
        matches!(self, Self::MacroCall(..))
    }

    /// Test if the item has any attributes
    pub fn has_unsupported_attributes(&self) -> bool {
        match self {
            Item::ItemUse(item) => !item.attributes.is_empty(),
            Item::ItemFn(item) => !item.attributes.is_empty(),
            Item::ItemEnum(item) => !item.attributes.is_empty(),
            Item::ItemStruct(item) => !item.attributes.is_empty(),
            Item::ItemImpl(item) => !item.attributes.is_empty(),
            Item::ItemMod(item) => !item.attributes.is_empty(),
            Item::ItemConst(item) => !item.attributes.is_empty(),
            Item::MacroCall(_) => false,
        }
    }

    /// Test if declaration is suitable inside of a block.
    pub fn peek_as_stmt(parser: &mut Parser<'_>) -> Result<bool, ParseError> {
        let tokens = parser.token_peek_pair()?;
        let (t1, t2) = peek!(tokens, Ok(false));

        let kind = if matches!(t1.kind, ast::Kind::Pub) {
            peek!(t2, Ok(false)).kind
        } else {
            t1.kind
        };

        Ok(match kind {
            ast::Kind::Use => true,
            ast::Kind::Enum => true,
            ast::Kind::Struct => true,
            ast::Kind::Impl => true,
            ast::Kind::Async => matches!(peek!(t2, Ok(false)).kind, ast::Kind::Fn),
            ast::Kind::Fn => true,
            ast::Kind::Mod => true,
            ast::Kind::Const => true,
            _ => false,
        })
    }

    /// Parse an Item attaching the given Attributes
    pub fn parse_with_attributes(
        parser: &mut Parser,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        let t = parser.token_peek_eof()?;

        let kind = if t.kind == ast::Kind::Pub {
            let t2 = parser.token_peek2_eof()?;
            t2.kind
        } else {
            t.kind
        };

        Ok(match kind {
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
            ast::Kind::Const => {
                Self::ItemConst(ast::ItemConst::parse_with_attributes(parser, attributes)?)
            }
            ast::Kind::Ident(..) => Self::MacroCall(parser.parse()?),
            _ => {
                return Err(ParseError::new(
                    t,
                    ParseErrorKind::ExpectedItem { actual: kind },
                ))
            }
        })
    }
}

impl Parse for Item {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let attributes: Vec<ast::Attribute> = parser.parse()?;
        Self::parse_with_attributes(parser, attributes)
    }
}
