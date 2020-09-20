use crate::ast;
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// An object pattern.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct PatObject {
    /// The identifier of the object pattern.
    pub ident: ast::LitObjectIdent,
    /// The open object marker.
    pub open: ast::OpenBrace,
    /// The fields matched against.
    pub fields: Vec<(PatObjectItem, Option<ast::Comma>)>,
    /// Indicates if the pattern is open or not.
    pub open_pattern: Option<ast::DotDot>,
    /// The close brace.
    pub close: ast::CloseBrace,
}

impl PatObject {
    /// Parse the object with an opening path.
    pub fn parse_with_ident(
        parser: &mut Parser<'_>,
        ident: ast::LitObjectIdent,
    ) -> Result<Self, ParseError> {
        let open = parser.parse()?;
        let mut fields = Vec::new();

        let mut is_open = true;

        while !parser.peek::<ast::CloseBrace>()? && !parser.peek::<ast::DotDot>()? {
            let item = parser.parse()?;

            let comma = if parser.peek::<ast::Comma>()? {
                Some(parser.parse()?)
            } else {
                None
            };

            is_open = comma.is_some();
            fields.push((item, comma));

            if !is_open {
                break;
            }
        }

        let open_pattern = if is_open && parser.peek::<ast::DotDot>()? {
            Some(parser.parse()?)
        } else {
            None
        };

        let close = parser.parse()?;

        Ok(Self {
            ident,
            open,
            fields,
            close,
            open_pattern,
        })
    }
}

impl Parse for PatObject {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let ident = parser.parse()?;
        Self::parse_with_ident(parser, ident)
    }
}

/// An object item.
#[derive(Debug, Clone, ToTokens, Spanned, Parse)]
pub struct PatObjectItem {
    /// The key of an object.
    pub key: ast::LitObjectKey,
    /// The binding used for the pattern object.
    #[rune(iter)]
    pub binding: Option<(ast::Colon, ast::Pat)>,
}
