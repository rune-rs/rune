use crate::ast;
use crate::{Parse, ParseError, ParseErrorKind, Parser, Peek, Resolve, Spanned, Storage, ToTokens};
use runestick::Source;
use std::borrow::Cow;

/// An identifier, like `foo` or `Hello`.".
#[derive(Debug, Clone, Copy, ToTokens, Spanned)]
pub struct Ident {
    /// The kind of the identifier.
    pub token: ast::Token,
    /// The kind of the identifier.
    #[rune(skip)]
    pub kind: ast::StringSource,
}

impl Parse for Ident {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        match token.kind {
            ast::Kind::Ident(kind) => Ok(Self { token, kind }),
            _ => Err(ParseError::new(
                token,
                ParseErrorKind::TokenMismatch {
                    expected: ast::Kind::Ident(ast::StringSource::Text),
                    actual: token.kind,
                },
            )),
        }
    }
}

impl Peek for Ident {
    fn peek(p1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        match p1 {
            Some(p1) => matches!(p1.kind, ast::Kind::Ident(..)),
            _ => false,
        }
    }
}

impl<'a> Resolve<'a> for Ident {
    type Output = Cow<'a, str>;

    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<Cow<'a, str>, ParseError> {
        let span = self.token.span();

        match self.kind {
            ast::StringSource::Text => {
                let ident = source
                    .source(span)
                    .ok_or_else(|| ParseError::new(span, ParseErrorKind::BadSlice))?;

                Ok(Cow::Borrowed(ident))
            }
            ast::StringSource::Synthetic(id) => {
                let ident = storage.get_string(id).ok_or_else(|| {
                    ParseError::new(span, ParseErrorKind::BadSyntheticId { kind: "label", id })
                })?;

                Ok(Cow::Owned(ident))
            }
        }
    }
}

impl std::convert::From<ast::Super> for Ident {
    fn from(super_: ast::Super) -> Self {
        Ident {
            token: super_.token,
            kind: ast::StringSource::Text,
        }
    }
}

impl std::convert::From<ast::Crate> for Ident {
    fn from(crate_: ast::Crate) -> Self {
        Ident {
            token: crate_.token,
            kind: ast::StringSource::Text,
        }
    }
}

impl std::convert::From<ast::Underscore> for Ident {
    fn from(underscore: ast::Underscore) -> Self {
        Ident {
            token: underscore.token,
            kind: ast::StringSource::Text,
        }
    }
}

impl std::convert::From<ast::Extern> for Ident {
    fn from(extern_: ast::Extern) -> Self {
        Ident {
            token: extern_.token,
            kind: ast::StringSource::Text,
        }
    }
}
