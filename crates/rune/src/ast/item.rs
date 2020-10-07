use crate::ast;
use crate::{OptionSpanned as _, Parse, ParseError, Parser, Peeker, Spanned, ToTokens};

/// A declaration.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum Item {
    /// A use declaration.
    ItemUse(ast::ItemUse),
    /// A function declaration.
    // large variant, so boxed
    ItemFn(ast::ItemFn),
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

    /// Take the attributes associated with the item.
    pub fn take_attributes(&mut self) -> Vec<ast::Attribute> {
        use std::mem::take;

        match self {
            Item::ItemUse(item) => take(&mut item.attributes),
            Item::ItemFn(item) => take(&mut item.attributes),
            Item::ItemEnum(item) => take(&mut item.attributes),
            Item::ItemStruct(item) => take(&mut item.attributes),
            Item::ItemImpl(item) => take(&mut item.attributes),
            Item::ItemMod(item) => take(&mut item.attributes),
            Item::ItemConst(item) => take(&mut item.attributes),
            Item::MacroCall(item) => take(&mut item.attributes),
        }
    }

    /// Test if the item has any attributes
    pub fn attributes(&self) -> &[ast::Attribute] {
        match self {
            Item::ItemUse(item) => &item.attributes,
            Item::ItemFn(item) => &item.attributes,
            Item::ItemEnum(item) => &item.attributes,
            Item::ItemStruct(item) => &item.attributes,
            Item::ItemImpl(item) => &item.attributes,
            Item::ItemMod(item) => &item.attributes,
            Item::ItemConst(item) => &item.attributes,
            Item::MacroCall(item) => &item.attributes,
        }
    }

    /// Test if declaration is suitable inside of a file.
    pub fn peek_as_item(p: &mut Peeker<'_>, path: Option<&ast::Path>) -> bool {
        if path.is_some() {
            // Macro call.
            return matches!(p.nth(0), K![!]);
        }

        match p.nth(0) {
            K![use] => true,
            K![enum] => true,
            K![struct] => true,
            K![impl] => true,
            K![async] => matches!(p.nth(1), K![fn]),
            K![fn] => true,
            K![mod] => true,
            K![const] => true,
            _ => false,
        }
    }

    /// Parse an Item attaching the given meta and optional path.
    pub fn parse_with_meta_path(
        p: &mut Parser<'_>,
        mut attributes: Vec<ast::Attribute>,
        mut visibility: ast::Visibility,
        path: Option<ast::Path>,
    ) -> Result<Self, ParseError> {
        use std::mem::take;

        let item = if let Some(path) = path {
            Self::MacroCall(ast::MacroCall::parse_with_meta_path(
                p,
                take(&mut attributes),
                path,
            )?)
        } else {
            let mut const_token = p.parse::<Option<T![const]>>()?;
            let mut async_token = p.parse::<Option<T![async]>>()?;

            let item = match p.nth(0)? {
                K![use] => Self::ItemUse(ast::ItemUse::parse_with_meta(
                    p,
                    take(&mut attributes),
                    take(&mut visibility),
                )?),
                K![enum] => Self::ItemEnum(ast::ItemEnum::parse_with_meta(
                    p,
                    take(&mut attributes),
                    take(&mut visibility),
                )?),
                K![struct] => Self::ItemStruct(ast::ItemStruct::parse_with_meta(
                    p,
                    take(&mut attributes),
                    take(&mut visibility),
                )?),
                K![impl] => Self::ItemImpl(ast::ItemImpl::parse_with_attributes(
                    p,
                    take(&mut attributes),
                )?),
                K![fn] => Self::ItemFn(ast::ItemFn::parse_with_meta(
                    p,
                    take(&mut attributes),
                    take(&mut visibility),
                    take(&mut const_token),
                    take(&mut async_token),
                )?),
                K![mod] => Self::ItemMod(ast::ItemMod::parse_with_meta(
                    p,
                    take(&mut attributes),
                    take(&mut visibility),
                )?),
                K![ident] => {
                    if let Some(const_token) = const_token.take() {
                        Self::ItemConst(ast::ItemConst::parse_with_meta(
                            p,
                            take(&mut attributes),
                            take(&mut visibility),
                            const_token,
                        )?)
                    } else {
                        Self::MacroCall(p.parse()?)
                    }
                }
                _ => {
                    return Err(ParseError::expected(
                        &p.token(0)?,
                        "`fn`, `mod`, `struct`, `enum`, `use`, or macro call",
                    ))
                }
            };

            if let Some(span) = const_token.option_span() {
                return Err(ParseError::unsupported(span, "const modifier"));
            }

            if let Some(span) = async_token.option_span() {
                return Err(ParseError::unsupported(span, "async modifier"));
            }

            item
        };

        if let Some(span) = attributes.option_span() {
            return Err(ParseError::unsupported(span, "attribute"));
        }

        if let Some(span) = visibility.option_span() {
            return Err(ParseError::unsupported(span, "visibility modifier"));
        }

        Ok(item)
    }
}

impl Parse for Item {
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        let attributes = p.parse()?;
        let visibility = p.parse()?;
        let path = p.parse()?;
        Self::parse_with_meta_path(p, attributes, visibility, path)
    }
}
