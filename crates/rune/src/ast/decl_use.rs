use crate::ast;
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;
use runestick::Span;

/// An imported declaration.
#[derive(Debug, Clone)]
pub struct DeclUse {
    /// The use token.
    pub use_: ast::Use,
    /// The name of the imported module.
    pub path: ast::Path,
}

impl DeclUse {
    /// Get the span for the declaration.
    pub fn span(&self) -> Span {
        self.use_.span().join(self.path.span())
    }
}

/// Parsing an use declaration.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> rune::Result<()> {
/// parse_all::<ast::DeclUse>("use foo;")?;
/// parse_all::<ast::DeclUse>("use foo::bar;")?;
/// parse_all::<ast::DeclUse>("use foo::bar::baz;")?;
/// # Ok(())
/// # }
/// ```
impl Parse for DeclUse {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            use_: parser.parse()?,
            path: parser.parse()?,
        })
    }
}
