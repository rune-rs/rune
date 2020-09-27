use crate::ast;
use crate::{OptionSpanned as _, Parse, ParseError, ParseErrorKind, Parser, Spanned, ToTokens};

/// A declaration.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
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
        match self {
            Self::ItemUse(..) => true,
            Self::ItemStruct(st) => st.needs_semi_colon(),
            Self::ItemConst(..) => true,
            _ => false,
        }
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

    /// Test if declaration is suitable inside of a file.
    pub fn peek_as_item(
        parser: &mut Parser<'_>,
        path: Option<&ast::Path>,
    ) -> Result<bool, ParseError> {
        let (t1, t2) = peek!(parser.token_peek_pair()?, Ok(false));

        if path.is_some() {
            // Macro call.
            return Ok(matches!(t1.kind, ast::Kind::Bang));
        }

        Ok(match t1.kind {
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

    /// Parse an Item attaching the given meta and optional path.
    pub fn parse_with_meta_path(
        parser: &mut Parser,
        mut attributes: Vec<ast::Attribute>,
        mut visibility: ast::Visibility,
        path: Option<ast::Path>,
    ) -> Result<Self, ParseError> {
        use std::mem::take;

        let item = if let Some(path) = path {
            Self::MacroCall(ast::MacroCall::parse_with_meta_path(
                parser,
                take(&mut attributes),
                path,
            )?)
        } else {
            let t = parser.token_peek_eof()?;

            match t.kind {
                ast::Kind::Use => Self::ItemUse(ast::ItemUse::parse_with_meta(
                    parser,
                    take(&mut attributes),
                    take(&mut visibility),
                )?),
                ast::Kind::Enum => Self::ItemEnum(ast::ItemEnum::parse_with_meta(
                    parser,
                    take(&mut attributes),
                    take(&mut visibility),
                )?),
                ast::Kind::Struct => Self::ItemStruct(ast::ItemStruct::parse_with_meta(
                    parser,
                    take(&mut attributes),
                    take(&mut visibility),
                )?),
                ast::Kind::Impl => Self::ItemImpl(ast::ItemImpl::parse_with_attributes(
                    parser,
                    take(&mut attributes),
                )?),
                ast::Kind::Async | ast::Kind::Fn => {
                    Self::ItemFn(Box::new(ast::ItemFn::parse_with_meta(
                        parser,
                        take(&mut attributes),
                        take(&mut visibility),
                    )?))
                }
                ast::Kind::Mod => Self::ItemMod(ast::ItemMod::parse_with_meta(
                    parser,
                    take(&mut attributes),
                    take(&mut visibility),
                )?),
                ast::Kind::Const => Self::ItemConst(ast::ItemConst::parse_with_meta(
                    parser,
                    take(&mut attributes),
                    take(&mut visibility),
                )?),
                ast::Kind::Ident(..) => Self::MacroCall(parser.parse()?),
                _ => {
                    return Err(ParseError::expected(
                        t,
                        "`fn`, `mod`, `struct`, `enum`, `use`, or macro call",
                    ))
                }
            }
        };

        if let Some(span) = attributes.option_span() {
            return Err(ParseError::new(
                span,
                ParseErrorKind::UnsupportedItemAttributes,
            ));
        }

        if let Some(span) = visibility.option_span() {
            return Err(ParseError::new(
                span,
                ParseErrorKind::UnsupportedItemVisibility,
            ));
        }

        Ok(item)
    }
}

impl Parse for Item {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        let visibility = parser.parse()?;
        let path = parser.parse()?;
        Self::parse_with_meta_path(parser, attributes, visibility, path)
    }
}
