use crate::ast;
use crate::{Parse, ParseError, ParseErrorKind, Parser, Peek, Resolve, Spanned, Storage, ToTokens};
use runestick::Source;
use std::borrow::Cow;

/// A path, where each element is separated by a `::`.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct Path {
    /// The optional leading colon `::`
    #[rune(iter)]
    pub leading_colon: Option<ast::Scope>,
    /// The first component in the path.
    pub first: ast::Ident,
    /// The rest of the components in the path.
    #[rune(iter)]
    pub rest: Vec<(ast::Scope, ast::Ident)>,
    /// Trailing scope.
    #[rune(iter)]
    pub trailing: Option<ast::Scope>,
}

impl Path {
    /// Borrow as an identifier used for field access calls.
    ///
    /// This is only allowed if there are no other path components.
    pub fn try_as_ident(&self) -> Option<&ast::Ident> {
        if self.rest.is_empty() && self.trailing.is_none() {
            Some(&self.first)
        } else {
            None
        }
    }

    /// Iterate over all components in path.
    pub fn into_components(&self) -> impl Iterator<Item = &'_ ast::Ident> + '_ {
        let mut first = Some(&self.first);
        let mut it = self.rest.iter();

        std::iter::from_fn(move || {
            if let Some(first) = first.take() {
                return Some(first);
            }

            Some(&it.next()?.1)
        })
    }
}

impl Peek for Path {
    fn peek(t1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        matches!(peek!(t1).kind, ast::Kind::Ident(..))
    }
}

/// Parsing Paths
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast, ParseError};
///
/// parse_all::<ast::Path>("x").unwrap();
/// parse_all::<ast::Path>("::x").unwrap();
/// parse_all::<ast::Path>("a::b").unwrap();
/// parse_all::<ast::Path>("::ab::cd").unwrap();
/// parse_all::<ast::Path>("crate").unwrap();
/// parse_all::<ast::Path>("super").unwrap();
/// parse_all::<ast::Path>("crate::foo").unwrap();
/// parse_all::<ast::Path>("super::bar").unwrap();
/// parse_all::<ast::Path>("::super").unwrap();
/// parse_all::<ast::Path>("::crate").unwrap();
/// ```
///
impl Parse for Path {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let leading_colon = parser.parse::<Option<ast::Scope>>()?;

        let token = parser.token_peek_eof()?;

        let first: ast::Ident = match token.kind {
            ast::Kind::Ident(_) => parser.parse()?,
            ast::Kind::Super => parser.parse::<ast::Super>()?.into(),
            ast::Kind::Crate => parser.parse::<ast::Crate>()?.into(),
            _ => {
                return Err(ParseError::new(
                    token,
                    ParseErrorKind::TokenMismatch {
                        expected: ast::Kind::Ident(ast::StringSource::Text),
                        actual: token.kind,
                    },
                ))
            }
        };

        Ok(Self {
            leading_colon,
            first,
            rest: parser.parse()?,
            trailing: parser.parse()?,
        })
    }
}

impl<'a> Resolve<'a> for Path {
    type Output = Vec<Cow<'a, str>>;

    fn resolve(
        &self,
        storage: &Storage,
        source: &'a Source,
    ) -> Result<Vec<Cow<'a, str>>, ParseError> {
        let mut output = Vec::new();

        output.push(self.first.resolve(storage, source)?);

        for (_, ident) in &self.rest {
            output.push(ident.resolve(storage, source)?);
        }

        Ok(output)
    }
}
