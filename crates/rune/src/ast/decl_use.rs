use crate::ast;
use crate::ast::Kind;
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::{Parse, Peek};
use runestick::Span;

/// An imported declaration.
#[derive(Debug, Clone)]
pub struct DeclUse {
    /// The use token.
    pub use_: ast::Use,
    /// First component in use.
    pub first: ast::Ident,
    /// The rest of the import.
    pub rest: Vec<(ast::Scope, DeclUseComponent)>,
}

impl DeclUse {
    /// Get the span for the declaration.
    pub fn span(&self) -> Span {
        if let Some((_, last)) = self.rest.last() {
            self.use_.span().join(last.span())
        } else {
            self.use_.span().join(self.first.span())
        }
    }
}

/// Parsing an use declaration.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::DeclUse>("use foo;").unwrap();
/// parse_all::<ast::DeclUse>("use foo::bar;").unwrap();
/// parse_all::<ast::DeclUse>("use foo::bar::baz;").unwrap();
/// ```
impl Parse for DeclUse {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            use_: parser.parse()?,
            first: parser.parse()?,
            rest: parser.parse()?,
        })
    }
}

/// A use component.
#[derive(Debug, Clone)]
pub enum DeclUseComponent {
    /// An identifier import.
    Ident(ast::Ident),
    /// A wildcard import.
    Wildcard(ast::Mul),
}

impl DeclUseComponent {
    /// Get the span for the declaration.
    pub fn span(&self) -> Span {
        match self {
            Self::Ident(ident) => ident.span(),
            Self::Wildcard(wildcard) => wildcard.span(),
        }
    }
}

impl Parse for DeclUseComponent {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let t = parser.token_peek_eof()?;

        Ok(match t.kind {
            ast::Kind::Ident => Self::Ident(parser.parse()?),
            ast::Kind::Star => Self::Wildcard(parser.parse()?),
            actual => {
                return Err(ParseError::ExpectedDeclUseImportComponent {
                    span: t.span,
                    actual,
                })
            }
        })
    }
}

impl Peek for DeclUseComponent {
    fn peek(t1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        let kind = match t1 {
            Some(t) => t.kind,
            None => return false,
        };

        matches!(kind, Kind::Ident | Kind::Star)
    }
}
