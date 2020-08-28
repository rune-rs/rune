use crate::ast;
use crate::parser::Parser;
use crate::token::Kind;
use crate::traits::{Parse, Peek};
use runestick::Span;

/// A declaration.
#[derive(Debug, Clone)]
pub enum Decl {
    /// A use declaration.
    DeclUse(ast::DeclUse),
    /// A function declaration.
    DeclFn(ast::DeclFn),
    /// An enum declaration.
    DeclEnum(ast::DeclEnum),
    /// A struct declaration.
    DeclStruct(ast::DeclStruct),
}

impl Decl {
    /// The span of the declaration.
    pub fn span(&self) -> Span {
        match self {
            Self::DeclUse(decl_use) => decl_use.span(),
            Self::DeclFn(decl_fn) => decl_fn.span(),
            Self::DeclEnum(decl_enum) => decl_enum.span(),
            Self::DeclStruct(decl_struct) => decl_struct.span(),
        }
    }

    /// Indicates if the declaration needs a semi-colon or not.
    pub fn needs_semi_colon(&self) -> bool {
        match self {
            Self::DeclUse(..) => true,
            Self::DeclFn(..) => false,
            Self::DeclEnum(..) => false,
            Self::DeclStruct(decl_struct) => decl_struct.needs_semi_colon(),
        }
    }
}

impl Peek for Decl {
    fn peek(t1: Option<crate::Token>, _: Option<crate::Token>) -> bool {
        let t1 = match t1 {
            Some(t1) => t1,
            None => return false,
        };

        match t1.kind {
            Kind::Use => true,
            Kind::Enum => true,
            Kind::Struct => true,
            Kind::Fn => true,
            _ => false,
        }
    }
}

impl Parse for Decl {
    fn parse(parser: &mut Parser) -> crate::Result<Self, crate::ParseError> {
        Ok(match parser.token_peek_eof()?.kind {
            Kind::Use => Self::DeclUse(parser.parse()?),
            Kind::Enum => Self::DeclEnum(parser.parse()?),
            Kind::Struct => Self::DeclStruct(parser.parse()?),
            _ => Self::DeclFn(parser.parse()?),
        })
    }
}
